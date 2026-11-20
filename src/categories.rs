use clap::ValueEnum;

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum Surface {
    System,
    Profile,
}

impl Surface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Profile => "profile",
        }
    }
}

pub struct Category {
    pub id: &'static str,
    pub label: &'static str,
    pub surface: Surface,
    path: &'static str,
    subtree: bool,
}
use Surface::{Profile, System};
const fn rule(
    id: &'static str,
    label: &'static str,
    surface: Surface,
    path: &'static str,
    subtree: bool,
) -> Category {
    Category {
        id,
        label,
        surface,
        path,
        subtree,
    }
}
// First match wins: shared startup must precede its Start menu parent.
pub const CATEGORIES: &[Category] = &[
    rule(
        "system-registry",
        "system registry",
        System,
        "windows/system32/config",
        true,
    ),
    rule(
        "tasks",
        "scheduled tasks",
        System,
        "windows/system32/tasks",
        true,
    ),
    rule(
        "drivers",
        "drivers",
        System,
        "windows/system32/drivers",
        true,
    ),
    rule(
        "wmi",
        "WMI repository",
        System,
        "windows/system32/wbem/repository",
        true,
    ),
    rule(
        "policy",
        "local group policy",
        System,
        "windows/system32/grouppolicy",
        true,
    ),
    rule(
        "policy-users",
        "local user group policy",
        System,
        "windows/system32/grouppolicyusers",
        true,
    ),
    rule(
        "shared-startup",
        "shared startup",
        System,
        "programdata/microsoft/windows/start menu/programs/startup",
        true,
    ),
    rule(
        "shared-programs",
        "shared Start menu",
        System,
        "programdata/microsoft/windows/start menu/programs",
        true,
    ),
    rule(
        "user-registry",
        "user registry",
        Profile,
        "ntuser.dat",
        false,
    ),
    rule(
        "user-classes",
        "user classes registry",
        Profile,
        "appdata/local/microsoft/windows/usrclass.dat",
        false,
    ),
    rule("downloads", "downloads", Profile, "downloads", true),
    rule(
        "user-startup",
        "user startup",
        Profile,
        "appdata/roaming/microsoft/windows/start menu/programs/startup",
        true,
    ),
];

pub fn classify(relative: &str) -> Option<&'static Category> {
    let lower = relative.to_ascii_lowercase();
    let (surface, tail) = match lower.strip_prefix("users/") {
        Some(profile) => (Profile, profile.split_once('/')?.1),
        None => (System, lower.as_str()),
    };
    CATEGORIES.iter().find(|c| {
        c.surface == surface
            && tail
                .strip_prefix(c.path)
                .is_some_and(|suffix| suffix.is_empty() || (c.subtree && suffix.starts_with('/')))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_files_and_subtrees() {
        for c in CATEGORIES {
            let path = if c.surface == Profile {
                format!("Users/Alice/{}", c.path)
            } else {
                c.path.to_owned()
            };
            assert_eq!(classify(&path.to_uppercase()).unwrap().id, c.id);
            assert_ne!(classify(&format!("{path}-old")).map(|c| c.id), Some(c.id));
            assert_eq!(classify(&format!("{path}/child")).is_some(), c.subtree);
        }
    }

    #[test]
    fn shared_startup_precedes_shared_programs() {
        let startup = CATEGORIES.iter().position(|c| c.id == "shared-startup");
        let programs = CATEGORIES.iter().position(|c| c.id == "shared-programs");
        assert!(startup.expect("shared-startup") < programs.expect("shared-programs"));
        assert_eq!(
            classify("ProgramData/Microsoft/Windows/Start Menu/Programs/Startup/run.lnk")
                .map(|c| c.id),
            Some("shared-startup")
        );
    }

    #[test]
    fn surface_names_match_clap() {
        for surface in [Surface::System, Surface::Profile] {
            assert_eq!(
                surface.as_str(),
                surface
                    .to_possible_value()
                    .expect("ValueEnum name")
                    .get_name()
            );
        }
    }

    #[test]
    fn group_policy_users_is_distinct() {
        assert_eq!(
            classify("Windows/System32/GroupPolicy/Machine").map(|c| c.id),
            Some("policy")
        );
        assert_eq!(
            classify("Windows/System32/GroupPolicyUsers/S-1-5-21").map(|c| c.id),
            Some("policy-users")
        );
    }
}
