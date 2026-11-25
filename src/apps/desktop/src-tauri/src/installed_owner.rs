use std::path::{Path, PathBuf};

pub(super) fn installed_owner(configured: Option<&str>, running: &Path) -> Option<PathBuf> {
    let path = Path::new(configured?);
    // Runtime checks the physical path and rejects transient builds again
    // before it writes client configuration.
    (path.is_absolute()
        && path == running
        && !path.components().any(|part| part.as_os_str() == "target"))
    .then(|| path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALLED: &str = r"F:\Applications\Agent HUD\Agent HUD.exe";
    const BUILD: &str = r"F:\Applications\agent-hud-open\target\release\Agent HUD.exe";

    #[test]
    fn only_the_explicitly_configured_installed_copy_can_be_the_owner() {
        assert_eq!(
            installed_owner(Some(INSTALLED), Path::new(INSTALLED)),
            Some(PathBuf::from(INSTALLED)),
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
