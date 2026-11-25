//! Tests for same-project auto-start and task-isolated fresh-session guardrails.
use super::*;

fn prefer_fresh_world() -> World {
    World::new(Preferences {
        enabled: true,
        require_reply: true,
        prefer_new_session: true,
        ..every_message_asks()
    })
}

#[test]
fn start_folders_include_callers_current_project_even_when_target_client_never_opened_it() {
    let mut world = prefer_fresh_world();
    let fresh_project = world.fixture.home().join("fresh-game");
    std::fs::create_dir_all(&fresh_project).unwrap();
    world.snapshot["sessions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": ANTIGRAVITY,
            "source": "antigravity",
            "title": "Remaster game",
            "cwd": fresh_project,
            "state": "running",
            "parentId": null,
            "updatedAt": "2026-10-22T01:05:00Z",
            "liveFacts": {}
        }));
    // Codex only ever worked in `~/project`, never in `~/fresh-game`.
    let listed = world.call(
        Some(("antigravity", ANTIGRAVITY)),
        "list_sessions",
        json!({}),
    );
    let codex_folders = listed["startFolders"]["codex"]["folders"]
        .as_array()
        .unwrap();
    assert_eq!(codex_folders[0], "~/fresh-game", "{listed}");
    assert!(codex_folders.iter().any(|f| f == "~/project"));
}

#[test]
fn start_session_accepts_callers_project_and_normalizes_explicit_absolute_paths() {
    let mut world = prefer_fresh_world();
    let fresh_project = world.fixture.home().join("fresh-game");
    std::fs::create_dir_all(&fresh_project).unwrap();
    world.snapshot["sessions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": ANTIGRAVITY,
            "source": "antigravity",
            "title": "Remaster game",
            "cwd": fresh_project,
            "state": "running",
            "parentId": null,
            "updatedAt": "2026-10-22T01:05:00Z",
            "liveFacts": {}
        }));
    // Trust the new folder for Claude so the fake Claude runner starts cleanly.
    let claude_json = world.fixture.home().join(".claude.json");
    let mut settings: Value =
        serde_json::from_slice(&std::fs::read(&claude_json).unwrap()).unwrap();
    settings["projects"][fresh_project.to_string_lossy().as_ref()] =
        json!({"hasTrustDialogAccepted": true});
    std::fs::write(&claude_json, settings.to_string()).unwrap();

    // Omitting folder defaults to caller's own project, even though Claude never opened it.
    let started = world.call(
        Some(("antigravity", ANTIGRAVITY)),
        "start_session",
        json!({"client": "claude", "text": "Inspect the game binary"}),
    );
    assert_eq!(started["state"], "started", "{started}");
    assert_eq!(started["sessionId"], STARTED);

    // Passing the absolute path directly also normalizes to `~/fresh-game` and succeeds.
    let started_abs = world.call(
        Some(("antigravity", ANTIGRAVITY)),
        "start_session",
        json!({
            "client": "claude",
            "folder": fresh_project.to_string_lossy(),
            "text": "Inspect save files"
        }),
    );
    assert_eq!(started_abs["state"], "started", "{started_abs}");
}

#[test]
fn prefer_new_session_refuses_cold_send_to_old_or_other_project_sessions_and_allows_followups() {
    let mut world = prefer_fresh_world();
    let other_project = world.fixture.home().join("other-project");
    std::fs::create_dir_all(&other_project).unwrap();
    world.snapshot["sessions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": ANTIGRAVITY,
            "source": "antigravity",
            "title": "New task",
            "cwd": other_project,
            "state": "running",
            "parentId": null,
            "updatedAt": "2026-10-22T01:05:00Z",
            "liveFacts": {}
        }));

    // Cold send across projects is refused and points to start_session.
    let cross = world.call(
        Some(("antigravity", ANTIGRAVITY)),
        "send_message",
        json!({"to": CODEX, "text": "Help with new task"}),
    );
    assert_eq!(cross["state"], "refused", "{cross}");
    assert!(
        cross["reason"]
            .as_str()
            .unwrap()
            .contains("different project"),
        "{cross}"
    );
    assert!(world.codex.0.lock().unwrap().is_empty());

    // Cold send within the same project is also refused so old context is not reused.
    let same = world.call(
        Some(("claude", CLAUDE)),
        "send_message",
        json!({"to": CODEX, "text": "Start a brand new task"}),
    );
    assert_eq!(same["state"], "refused", "{same}");
    assert!(
        same["reason"].as_str().unwrap().contains("start_session"),
        "{same}"
    );
    assert!(world.codex.0.lock().unwrap().is_empty());

    // Explicit reuse_existing: true (when user named a session) is allowed.
    let explicit = world.call(
        Some(("claude", CLAUDE)),
        "send_message",
        json!({"to": CODEX, "text": "Continue this user-chosen session", "reuse_existing": true}),
    );
    assert_eq!(explicit["state"], "queued", "{explicit}");

    // Once linked in the journal, subsequent follow-ups do not need reuse_existing.
    let followup = world.call(
        Some(("claude", CLAUDE)),
        "send_message",
        json!({"to": CODEX, "text": "Follow up on the same task"}),
    );
    assert_eq!(followup["state"], "queued", "{followup}");

    // Replies with reply_to are always allowed.
    let reply = world.call(
        Some(("codex", CODEX)),
        "send_message",
        json!({"reply_to": explicit["id"], "text": "Got it"}),
    );
    assert_eq!(reply["state"], "queued", "{reply}");
}

#[test]
fn prefer_new_session_allows_followup_to_a_session_started_by_start_session() {
    let mut world = prefer_fresh_world();
    let started = world.call(
        Some(("claude", CLAUDE)),
        "start_session",
        json!({"client": "claude", "text": "Run the suite"}),
    );
    assert_eq!(started["state"], "started", "{started}");
    let project = world.fixture.home().join("project");
    world.snapshot["sessions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": STARTED,
            "source": "claude",
            "title": "Suite",
            "cwd": project,
            "state": "idle",
            "parentId": null,
            "updatedAt": "2026-10-22T01:06:00Z",
            "liveFacts": {}
        }));
    world.check_in(STARTED);
    // Follow-up to the session we just started works without reuse_existing.
    let followup = world.call(
        Some(("claude", CLAUDE)),
        "send_message",
        json!({"to": STARTED, "text": "Also check lint"}),
    );
    assert_eq!(followup["state"], "queued", "{followup}");
}

#[test]
fn fresh_session_preference_defaults_on_and_shapes_tool_descriptions() {
    assert!(Preferences::read(&json!({})).prefer_new_session);
    assert!(
        !Preferences::read(&json!({"agentMessagesPreferNewSession": false})).prefer_new_session
    );
    let on = Messaging::tools(&Preferences {
        enabled: true,
        prefer_new_session: true,
        ..every_message_asks()
    });
    let send = on["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "send_message")
        .unwrap();
    assert!(send["inputSchema"]["properties"]["reuse_existing"].is_object());
    assert!(send["description"]
        .as_str()
        .unwrap()
        .contains("start_session"));
    let off = Messaging::tools(&Preferences {
        enabled: true,
        prefer_new_session: false,
        ..every_message_asks()
    });
    let send_off = off["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "send_message")
        .unwrap();
    assert!(send_off["inputSchema"]["properties"]["reuse_existing"].is_null());
}
