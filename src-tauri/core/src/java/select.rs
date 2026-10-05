use crate::java::detect::JavaRuntime;
use crate::mojang::profile::JavaRequirement;
use crate::text::UiText;

fn max_compatible_major(required: u32) -> u32 {
    if required <= 8 {
        11
    } else {
        u32::MAX
    }
}

fn best<'a>(mut candidates: impl Iterator<Item = &'a JavaRuntime>) -> Option<&'a JavaRuntime> {
    let first = candidates.next()?;
    if first.is_64bit {
        return Some(first);
    }

    let mut fallback = first;
    for candidate in candidates {
        if candidate.is_64bit {
            return Some(candidate);
        }
        fallback = fallback.min_by_path(candidate);
    }

    Some(fallback)
}

impl JavaRuntime {
    fn min_by_path<'a>(&'a self, other: &'a Self) -> &'a Self {
        if other.path < self.path {
            other
        } else {
            self
        }
    }
}

pub fn pick<'a>(
    runtimes: &'a [JavaRuntime],
    requirement: &JavaRequirement,
) -> Option<&'a JavaRuntime> {
    let major = requirement.major?;

    if requirement.at_least {
        return best(runtimes.iter().filter(|runtime| runtime.major >= major));
    }

    if let Some(exact) = best(runtimes.iter().filter(|runtime| runtime.major == major)) {
        return Some(exact);
    }

    let ceiling = max_compatible_major(major);
    let closest = runtimes
        .iter()
        .filter(|runtime| runtime.major > major && runtime.major <= ceiling)
        .map(|runtime| runtime.major)
        .min()?;

    best(runtimes.iter().filter(|runtime| runtime.major == closest))
}

pub fn pick_system(runtimes: &[JavaRuntime]) -> Option<&JavaRuntime> {
    runtimes
        .iter()
        .find(|runtime| matches!(runtime.source, "path" | "java_home"))
        .or_else(|| runtimes.first())
}

pub fn installed_majors(runtimes: &[JavaRuntime]) -> String {
    let mut majors: Vec<u32> = runtimes.iter().map(|runtime| runtime.major).collect();
    majors.sort_unstable();
    majors.dedup();

    if majors.is_empty() {
        return "none".to_string();
    }

    majors
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn requirement_text(requirement: &JavaRequirement, runtimes: &[JavaRuntime]) -> UiText {
    if runtimes.is_empty() {
        return UiText::new("error.reason.java.required_none")
            .param_text("required", requirement.text());
    }

    UiText::new("error.reason.java.required_other")
        .param_text("required", requirement.text())
        .param("installed", installed_majors(runtimes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(major: u32, is_64bit: bool, source: &'static str) -> JavaRuntime {
        JavaRuntime {
            path: format!("/jdk{major}{}/bin/java", if is_64bit { "" } else { "-32" }),
            version: format!("{major}.0.1"),
            major,
            vendor: "Test".into(),
            arch: if is_64bit {
                "x86_64".into()
            } else {
                "x86".into()
            },
            os_version: "10.0".into(),
            is_64bit,
            source,
        }
    }

    fn exact(major: u32) -> JavaRequirement {
        JavaRequirement {
            major: Some(major),
            component: None,
            at_least: false,
        }
    }

    #[test]
    fn exact_major_wins_over_higher() {
        let runtimes = vec![runtime(21, true, "system"), runtime(17, true, "system")];
        assert_eq!(pick(&runtimes, &exact(17)).unwrap().major, 17);
    }

    #[test]
    fn falls_back_to_closest_higher_within_limit() {
        let runtimes = vec![runtime(11, true, "system"), runtime(17, true, "system")];

        assert_eq!(pick(&runtimes, &exact(8)).unwrap().major, 11);
        assert!(pick(&[runtime(17, true, "system")], &exact(8)).is_none());
    }

    #[test]
    fn at_least_takes_anything_newer() {
        let runtimes = vec![runtime(17, true, "system"), runtime(24, true, "system")];

        let requirement = JavaRequirement {
            major: Some(21),
            component: None,
            at_least: true,
        };

        assert_eq!(pick(&runtimes, &requirement).unwrap().major, 24);
    }

    #[test]
    fn prefers_64bit_among_same_major() {
        let runtimes = vec![runtime(17, false, "system"), runtime(17, true, "system")];
        assert!(pick(&runtimes, &exact(17)).unwrap().is_64bit);
    }

    #[test]
    fn system_pick_prefers_path_and_java_home() {
        let runtimes = vec![runtime(8, true, "registry"), runtime(21, true, "java_home")];
        assert_eq!(pick_system(&runtimes).unwrap().major, 21);
        assert_eq!(pick_system(&runtimes[..1]).unwrap().major, 8);
        assert!(pick_system(&[]).is_none());
    }

    #[test]
    fn describes_what_is_installed() {
        let runtimes = vec![
            runtime(21, true, "system"),
            runtime(8, true, "system"),
            runtime(21, false, "system"),
        ];
        assert_eq!(installed_majors(&runtimes), "8, 21");
        assert_eq!(installed_majors(&[]), "none");
    }
}
