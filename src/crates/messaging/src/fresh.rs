//! Same-project session starts and task isolation: lets `start_session` open
//! in the caller's current project even when the target client has never
//! opened it before, and guards against reusing unrelated old sessions.
use super::*;
use agent_hud_platform::delivery::StartOffer;

pub(crate) fn same_folder(a: &str, b: &str) -> bool {
    delivery::same_folder_name(a, b)
}

fn match_offered<'a>(offer: &'a StartOffer, candidate: &str) -> Option<&'a str> {
    offer
        .folders
        .iter()
        .find(|folder| same_folder(folder, candidate))
        .map(String::as_str)
}

impl Messaging {
    /// Start offers for `caller`: includes the caller's own project folder at
    /// the front of every startable client that accepts it, so a new session
    /// can open in the same project even before that client has opened it.
    pub(crate) fn offers_for_caller(
        &self,
        caller: Option<&Caller>,
        host: &Host<'_>,
        extras: &[Value],
    ) -> Vec<StartOffer> {
        let mut offers = self.starts.offers_for(&all_sessions(host.snapshot));
        let Some(cwd) = caller.and_then(|c| self.caller_cwd(c, host, extras)) else {
            return offers;
        };
        for offer in &mut offers {
            let Some(written) = self.starts.accepted_folder(offer.provider, cwd) else {
                continue;
            };
            if let Some(pos) = offer
                .folders
                .iter()
                .position(|folder| same_folder(folder, &written))
            {
                if pos > 0 {
                    let existing = offer.folders.remove(pos);
                    offer.folders.insert(0, existing);
                }
            } else {
                offer.folders.insert(0, written);
            }
        }
        offers
    }

    pub(crate) fn caller_cwd<'a>(
        &self,
        caller: &Caller,
        host: &'a Host<'a>,
        extras: &'a [Value],
    ) -> Option<&'a str> {
        find_in_any(host.snapshot, extras, &caller.client, &caller.session_id)
            .and_then(|session| session["cwd"].as_str())
            .filter(|cwd| !cwd.is_empty())
    }

    /// Resolves `folder` from tool arguments (or defaults to caller's project),
    /// normalizing absolute paths to the offered spelling when they match.
    pub(crate) fn resolve_start_folder(
        &self,
        offer: &StartOffer,
        caller_cwd: Option<&str>,
        raw: Option<&str>,
    ) -> String {
        let candidate = match raw {
            Some(folder) if folder.trim().is_empty() => return String::new(),
            Some(folder) => self
                .starts
                .offered_folder(folder)
                .unwrap_or_else(|| folder.to_owned()),
            None => caller_cwd
                .and_then(|cwd| self.starts.offered_folder(cwd))
                .unwrap_or_default(),
        };
        if candidate.is_empty() {
            return candidate;
        }
        match_offered(offer, &candidate)
            .map(str::to_owned)
            .unwrap_or(candidate)
    }

    /// Guardrail for `prefer_new_session`: refuses a cold `send_message` to an
    /// existing startable-client session that has no prior exchange with the
    /// caller, unless it is a reply, an explicit steer into a turn in progress,
    /// or the caller explicitly passes `reuse_existing: true`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn check_fresh_session(
        &self,
        caller: &Caller,
        client: &str,
        to: &str,
        target: &Value,
        skip_for_reply_or_steer: bool,
        arguments: &Value,
        host: &Host<'_>,
        extras: &[Value],
        language: Language,
    ) -> Option<String> {
        if !self.preferences.prefer_new_session
            || skip_for_reply_or_steer
            || arguments["reuse_existing"] == true
            || !matches!(client, "claude" | "codex" | "antigravity" | "deepseek")
            || self
                .journal
                .exchanged((&caller.client, &caller.session_id), (client, to))
        {
            return None;
        }
        let caller_project = self
            .caller_cwd(caller, host, extras)
            .and_then(|cwd| self.starts.offered_folder(cwd));
        let target_project = target["cwd"]
            .as_str()
            .filter(|cwd| !cwd.is_empty())
            .and_then(|cwd| self.starts.offered_folder(cwd))
            .or_else(|| {
                target["project"]
                    .as_str()
                    .filter(|p| !p.is_empty())
                    .map(str::to_owned)
            });
        if let (Some(caller_proj), Some(target_proj)) = (caller_project, target_project) {
            if !same_folder(&caller_proj, &target_proj) {
                return Some(words::different_project(
                    language,
                    client,
                    &target_proj,
                    &caller_proj,
                ));
            }
        }
        Some(words::prefer_new_session(language, client))
    }

    /// How a message goes in, and whether its last line asks for a reply.
    /// The user's "Send as me" makes it their own words, except a `labelled`
    /// one (a steer). With "Ask for replies only when needed" the sender's
    /// `require_reply` decides the reply line, and a message that needs a
    /// reply is labelled under "Send as me" too: the recipient has to know
    /// whom it answers. Without it every labelled message asks.
    pub(crate) fn shape(&self, arguments: &Value, labelled: bool) -> (PromptForm, bool) {
        let ask = !self.preferences.require_reply || arguments["require_reply"] == true;
        let own = self.preferences.as_user && !labelled && !(self.preferences.require_reply && ask);
        if own {
            (PromptForm::Own, false)
        } else {
            (PromptForm::Labelled, ask)
        }
    }
}
