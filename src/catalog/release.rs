use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareRelease {
    pub version: String,
    pub object_key: String,
    pub size_bytes: i64,
    pub manifest_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemverParts {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

pub fn parse_semver(raw: &str) -> Option<SemverParts> {
    let raw = raw.trim();
    let s = raw
        .strip_prefix('v')
        .or_else(|| raw.strip_prefix('V'))
        .unwrap_or(raw);
    if s.is_empty() {
        return None;
    }
    let (ver_part, pre) = match s.split_once('-') {
        Some((v, p)) => {
            if p.trim().is_empty() {
                return None;
            }
            (v, Some(p.trim().to_owned()))
        }
        None => (s, None),
    };
    let parts: Vec<&str> = ver_part.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let major = parts[0].parse::<u64>().ok()?;
    let minor = if parts.len() > 1 {
        parts[1].parse::<u64>().ok()?
    } else {
        0
    };
    let patch = if parts.len() > 2 {
        parts[2].parse::<u64>().ok()?
    } else {
        0
    };
    Some(SemverParts {
        major,
        minor,
        patch,
        pre,
    })
}

pub fn compare_semver(a: &SemverParts, b: &SemverParts) -> Ordering {
    match (a.major, a.minor, a.patch).cmp(&(b.major, b.minor, b.patch)) {
        Ordering::Equal => match (&a.pre, &b.pre) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(pre_a), Some(pre_b)) => pre_a.cmp(pre_b),
        },
        other => other,
    }
}

pub fn parse_build_number(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    let s = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .or_else(|| trimmed.strip_prefix('#'))
        .or_else(|| trimmed.strip_prefix('b'))
        .or_else(|| trimmed.strip_prefix('B'))
        .unwrap_or(trimmed);
    s.parse::<u64>().ok()
}

pub fn channel_rank(channel: &str) -> u32 {
    match channel.to_ascii_lowercase().as_str() {
        "stable" => 100,
        "latest" => 90,
        "rc" | "release-candidate" => 80,
        "beta" => 50,
        "alpha" => 30,
        "dev" => 20,
        "nightly" => 10,
        _ => 0,
    }
}

pub fn extract_version_from_key(object_key: &str, software_id: &str) -> Option<String> {
    let trimmed_key = object_key.trim();
    let trimmed_software = software_id.trim();
    if trimmed_key.is_empty() || trimmed_software.is_empty() {
        return None;
    }

    let remainder = if let Some(suffix) = trimmed_key.strip_prefix(trimmed_software) {
        suffix
            .strip_prefix('/')
            .or_else(|| suffix.strip_prefix('-'))
            .unwrap_or(suffix)
    } else {
        return None;
    };

    if remainder.is_empty() {
        return None;
    }

    let version_part = if let Some((dir, _)) = remainder.split_once('/') {
        dir
    } else {
        let without_known_ext = remainder
            .strip_suffix(".tar.gz")
            .or_else(|| remainder.strip_suffix(".tar.xz"))
            .or_else(|| remainder.strip_suffix(".tar.bz2"))
            .unwrap_or(remainder);
        if let Some((stem, _)) = without_known_ext.rsplit_once('.') {
            if stem.is_empty() {
                without_known_ext
            } else {
                stem
            }
        } else {
            without_known_ext
        }
    };

    let cleaned = version_part.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_owned())
    }
}

pub fn is_newer(latest: &str, current: &str, scheme: &str) -> anyhow::Result<bool> {
    let norm_scheme = scheme.trim().to_ascii_uppercase();
    if norm_scheme == "DISABLED" {
        anyhow::bail!("release versioning scheme is disabled");
    }
    if latest.trim() == current.trim() {
        return Ok(false);
    }
    match norm_scheme.as_str() {
        "SEMVER" => {
            let parsed_latest = parse_semver(latest)
                .ok_or_else(|| anyhow::anyhow!("invalid semver in latest release: {latest}"))?;
            let parsed_current = parse_semver(current)
                .ok_or_else(|| anyhow::anyhow!("invalid semver in current version: {current}"))?;
            Ok(compare_semver(&parsed_latest, &parsed_current) == Ordering::Greater)
        }
        "BUILD_NUMBER" => {
            let parsed_latest = parse_build_number(latest).ok_or_else(|| {
                anyhow::anyhow!("invalid build number in latest release: {latest}")
            })?;
            let parsed_current = parse_build_number(current).ok_or_else(|| {
                anyhow::anyhow!("invalid build number in current version: {current}")
            })?;
            Ok(parsed_latest > parsed_current)
        }
        "CHANNEL" => {
            let rank_latest = channel_rank(latest);
            let rank_current = channel_rank(current);
            if rank_latest != rank_current {
                Ok(rank_latest > rank_current)
            } else {
                Ok(latest.trim() != current.trim())
            }
        }
        "TAG" => Ok(latest.trim() != current.trim()),
        "DISABLED" => anyhow::bail!("release versioning scheme is disabled"),
        other => anyhow::bail!("unsupported release versioning scheme: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_parsing_and_comparison_properties() {
        let v1 = parse_semver("1.0.0").expect("v1");
        let v2 = parse_semver("v1.0.1").expect("v2");
        let v3 = parse_semver("1.1.0").expect("v3");
        let v4 = parse_semver("2.0.0").expect("v4");
        let v_beta = parse_semver("1.0.0-beta.1").expect("v_beta");

        assert_eq!(compare_semver(&v1, &v1), Ordering::Equal);
        assert_eq!(compare_semver(&v1, &v2), Ordering::Less);
        assert_eq!(compare_semver(&v2, &v1), Ordering::Greater);
        assert_eq!(compare_semver(&v2, &v3), Ordering::Less);
        assert_eq!(compare_semver(&v3, &v4), Ordering::Less);
        assert_eq!(compare_semver(&v_beta, &v1), Ordering::Less);
        assert_eq!(compare_semver(&v1, &v_beta), Ordering::Greater);
    }

    #[test]
    fn semver_property_based_testing() {
        let mut seed: u64 = 987654321;
        let mut lcg = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            seed
        };

        for _ in 0..1000 {
            let major1 = lcg() % 100;
            let minor1 = lcg() % 100;
            let patch1 = lcg() % 100;
            let raw1 = format!("{major1}.{minor1}.{patch1}");

            let major2 = lcg() % 100;
            let minor2 = lcg() % 100;
            let patch2 = lcg() % 100;
            let raw2 = format!("{major2}.{minor2}.{patch2}");

            let p1 = parse_semver(&raw1).expect("p1");
            let p2 = parse_semver(&raw2).expect("p2");

            let tuple1 = (major1, minor1, patch1);
            let tuple2 = (major2, minor2, patch2);
            assert_eq!(compare_semver(&p1, &p2), tuple1.cmp(&tuple2));

            let newer = is_newer(&raw1, &raw2, "SEMVER").expect("newer check");
            if tuple1 > tuple2 {
                assert!(newer);
            } else {
                assert!(!newer);
            }
        }
    }

    #[test]
    fn build_number_property_based_testing() {
        let mut seed: u64 = 1122334455;
        let mut lcg = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            seed
        };

        for _ in 0..1000 {
            let b1 = lcg() % 10_000_000;
            let b2 = lcg() % 10_000_000;
            let raw1 = format!("v{b1}");
            let raw2 = format!("{b2}");

            let p1 = parse_build_number(&raw1).expect("p1");
            let p2 = parse_build_number(&raw2).expect("p2");
            assert_eq!(p1, b1);
            assert_eq!(p2, b2);

            let newer = is_newer(&raw1, &raw2, "BUILD_NUMBER").expect("newer check");
            assert_eq!(newer, b1 > b2);
        }
    }

    #[test]
    fn version_boundary_and_malformed_inputs() {
        assert!(parse_semver("").is_none());
        assert!(parse_semver("abc").is_none());
        assert!(parse_semver("1.2.3.4.5").is_none());
        assert!(parse_semver("1.2.-beta").is_none());

        assert!(parse_build_number("").is_none());
        assert!(parse_build_number("build-one").is_none());
        assert_eq!(parse_build_number("u64max"), None);

        assert!(is_newer("1.0.0", "invalid", "SEMVER").is_err());
        assert!(is_newer("100", "xyz", "BUILD_NUMBER").is_err());
        assert!(is_newer("1.0.0", "1.0.0", "DISABLED").is_err());
        assert_eq!(is_newer("1.0.0", "1.0.0", "SEMVER").unwrap(), false);
        assert_eq!(is_newer("500", "500", "BUILD_NUMBER").unwrap(), false);
    }

    #[test]
    fn extracts_version_from_key_shapes() {
        assert_eq!(
            extract_version_from_key("space-odyssey/v1.0.4/game.pmpack", "space-odyssey"),
            Some("v1.0.4".to_owned())
        );
        assert_eq!(
            extract_version_from_key("space-odyssey/1042/bin/game.exe", "space-odyssey"),
            Some("1042".to_owned())
        );
        assert_eq!(
            extract_version_from_key("space-odyssey/stable/game.zip", "space-odyssey"),
            Some("stable".to_owned())
        );
        assert_eq!(
            extract_version_from_key("space-odyssey/v1.2.0.pmpack", "space-odyssey"),
            Some("v1.2.0".to_owned())
        );
        assert_eq!(
            extract_version_from_key("space-odyssey/build-200.tar.gz", "space-odyssey"),
            Some("build-200".to_owned())
        );
        assert_eq!(
            extract_version_from_key("other-game/v1.0.0/game.pmpack", "space-odyssey"),
            None
        );
    }
}
