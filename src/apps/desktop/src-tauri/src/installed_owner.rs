use std::path::{Path, PathBuf};

pub(super) fn canonical_executable() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|running| {
        installed_owner(option_env!("AGENT_HUD_WINDOWS_INSTALLED_EXE"), &running)
    })
}

pub(super) fn installed_owner(configured: Option<&str>, running: &Path) -> Option<PathBuf> {
    let path = Path::new(configured?);
    let same = path
        .to_string_lossy()
        .eq_ignore_ascii_case(&running.to_string_lossy());
    let in_target = path.components().any(|part| {
        part.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("target")
    });
    // Runtime checks the physical path and rejects transient builds again
    // before it writes client configuration.
    (path.is_absolute() && same && !in_target).then(|| running.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALLED: &str = r"E:\Apps\Agent HUD\Agent HUD.exe";
    const INSTALLED_LOWER: &str = r"e:\apps\agent hud\agent hud.exe";
    const BUILD: &str = r"E:\agent-hud-open\target\release\Agent HUD.exe";

    #[test]
    fn only_the_explicitly_configured_installed_copy_can_be_the_owner() {
        assert_eq!(
            installed_owner(Some(INSTALLED), Path::new(INSTALLED)),
            Some(PathBuf::from(INSTALLED)),
        );
        assert_eq!(
            installed_owner(Some(INSTALLED), Path::new(INSTALLED_LOWER)),
            Some(PathBuf::from(INSTALLED_LOWER)),
        );
        assert_eq!(installed_owner(Some(INSTALLED), Path::new(BUILD)), None);
        assert_eq!(installed_owner(Some(BUILD), Path::new(BUILD)), None);
        assert_eq!(installed_owner(None, Path::new(INSTALLED)), None);
        assert_eq!(
            installed_owner(Some("Agent HUD.exe"), Path::new("Agent HUD.exe")),
            None,
        );
    }
}
