//! Fresh-session isolation: messages explaining why cold sends to old or
//! different-project sessions are refused in favour of `start_session`.
use agent_hud_core::l10n::{fill, tr_in, Language};

/// Target session is in a different project from the caller's.
pub fn different_project(
    language: Language,
    client: &str,
    target_project: &str,
    caller_project: &str,
) -> String {
    fill(
        &tr_in(
            language,
            "That {{client}} session belongs to a different project ({{target}} vs your {{caller}}). Use start_session to start a fresh session in your current project instead. If start_session cannot use the requested model, report to the user instead of reusing an old session; pass reuse_existing: true only when the user explicitly names this session.",
        ),
        &[
            ("client", &super::client(client)),
            ("target", &target_project),
            ("caller", &caller_project),
        ],
    )
}

/// Cold send to an existing session when the user prefers isolating tasks in
/// fresh sessions.
pub fn prefer_new_session(language: Language, client: &str) -> String {
    fill(
        &tr_in(
            language,
            "Tasks are isolated in separate sessions: use start_session to start a fresh {{client}} session in your current project instead of reusing an existing session's context. Pass reuse_existing: true only when the user explicitly told you to message this existing session.",
        ),
        &[("client", &super::client(client))],
    )
}

/// A new session cannot override the desktop app's model, and the requested
/// model differs from the client's current default.
pub fn model_at_start_default(
    language: Language,
    client: &str,
    asked: &str,
    default: &str,
) -> String {
    fill(
        &tr_in(
            language,
            "{{client}}'s desktop app runs a new session in its own chosen model (currently {{default}}), not {{asked}}. Tell the user so they can switch the model or open a new session for this project; do not silently reuse an old or different-project session.",
        ),
        &[
            ("client", &super::client(client)),
            ("default", &default),
            ("asked", &asked),
        ],
    )
}
