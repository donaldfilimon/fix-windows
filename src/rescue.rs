use crate::categories::Surface;

/// What an operator, or a model reading the report, should do with one file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// Personal file worth copying before any reinstall.
    Backup,
    /// Personal file whose type the recovery guide says not to copy blindly.
    Hold,
    /// Persistence or system metadata to review. Not a backup candidate.
    Persistence,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::Hold => "hold",
            Self::Persistence => "persistence",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Extra {
    pub role: Role,
    pub label: &'static str,
    pub surface: Surface,
}

const HOLD_EXT: &[&str] = &[
    "exe", "dll", "scr", "com", "msi", "msix", "bat", "cmd", "ps1", "vbs", "js", "jse", "wsf",
    "wsh", "sh", "docm", "xlsm", "pptm", "zip", "7z", "rar", "iso", "cab", "gz", "bz2", "xz",
    "dmg", "pkg",
];

const PROFILE_BACKUP: &[&str] = &[
    "documents",
    "desktop",
    "pictures",
    "videos",
    "movies",
    "music",
];

fn under(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

fn extension(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() || ext.is_empty() {
        None
    } else {
        Some(ext)
    }
}

fn hold_extension(path: &str) -> bool {
    extension(path).is_some_and(|ext| HOLD_EXT.contains(&ext))
}

fn profile_tail(lower: &str) -> Option<&str> {
    lower
        .strip_prefix("users/")
        .and_then(|rest| rest.split_once('/').map(|(_, tail)| tail))
}

/// Role for a path that already matched an inventory category.
pub fn category_role(category_id: &str, relative: &str, is_file: bool) -> Option<Role> {
    if !is_file {
        return None;
    }
    if category_id == "downloads" {
        Some(if hold_extension(&relative.to_ascii_lowercase()) {
            Role::Hold
        } else {
            Role::Backup
        })
    } else {
        Some(Role::Persistence)
    }
}

/// Personal files and Mac persistence paths that are not Windows inventory categories.
pub fn extra(relative: &str, is_file: bool) -> Option<Extra> {
    if !is_file {
        return None;
    }
    let lower = relative.to_ascii_lowercase();
    if let Some(tail) = profile_tail(&lower) {
        if under(tail, "library/launchagents") {
            return Some(Extra {
                role: Role::Persistence,
                label: "launch agent",
                surface: Surface::Profile,
            });
        }
        if PROFILE_BACKUP.iter().any(|dir| under(tail, dir)) {
            return Some(Extra {
                role: if hold_extension(&lower) {
                    Role::Hold
                } else {
                    Role::Backup
                },
                label: "personal file",
                surface: Surface::Profile,
            });
        }
        return None;
    }
    if under(&lower, "library/launchagents") || under(&lower, "library/launchdaemons") {
        return Some(Extra {
            role: Role::Persistence,
            label: if under(&lower, "library/launchdaemons") {
                "launch daemon"
            } else {
                "launch agent"
            },
            surface: Surface::System,
        });
    }
    if under(&lower, "library/startupitems") {
        return Some(Extra {
            role: Role::Persistence,
            label: "startup item",
            surface: Surface::System,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_hold_and_persistence() {
        assert_eq!(
            category_role("downloads", "Users/Alice/Downloads/example.txt", true),
            Some(Role::Backup)
        );
        assert_eq!(
            category_role("downloads", "Users/Alice/Downloads/setup.exe", true),
            Some(Role::Hold)
        );
        assert_eq!(
            category_role("tasks", "Windows/System32/Tasks/Example Task", true),
            Some(Role::Persistence)
        );
        assert_eq!(
            category_role("downloads", "Users/Alice/Downloads", false),
            None
        );
        let notes = extra("Users/Alice/Documents/notes.txt", true).expect("notes");
        assert_eq!(notes.role, Role::Backup);
        assert_eq!(notes.surface, Surface::Profile);
        let installer = extra("Users/Alice/Desktop/Setup.MSI", true).expect("msi");
        assert_eq!(installer.role, Role::Hold);
        assert!(extra("Users/Alice/Documents-old/notes.txt", true).is_none());
        let agent = extra("Users/Alice/Library/LaunchAgents/evil.plist", true).expect("agent");
        assert_eq!(agent.role, Role::Persistence);
        let daemon = extra("Library/LaunchDaemons/com.example.plist", true).expect("daemon");
        assert_eq!(daemon.label, "launch daemon");
        assert_eq!(daemon.surface, Surface::System);
    }
}
