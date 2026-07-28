#![forbid(unsafe_code)]

//! GitHub Release metadata: resolve the latest tag and compare versions.

use crate::domain::error::{MineError, MineResult};

/// The default release account/repo to query.
const DEFAULT_ACCOUNT: &str = "6ixGODD";
const DEFAULT_REPO: &str = "mine-is-not-everyones";

/// Resolves the latest published release tag (e.g. `v0.1.0`) from GitHub.
///
/// Honors `MINE_RELEASE_ACCOUNT` / `MINE_RELEASE_REPO` overrides for testing
/// and forks. Reads the real process environment only for those two names.
pub fn latest_tag() -> MineResult<String> {
    let account =
        std::env::var("MINE_RELEASE_ACCOUNT").unwrap_or_else(|_| DEFAULT_ACCOUNT.to_string());
    let repo = std::env::var("MINE_RELEASE_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string());
    let url = format!("https://api.github.com/repos/{account}/{repo}/releases/latest");
    let resp = ureq::get(&url)
        .header("User-Agent", "mine-setup")
        .header("Accept", "application/vnd.github+json")
        .call();
    match resp {
        Ok(r) => {
            let body: serde_json::Value =
                r.into_body().read_json().unwrap_or(serde_json::Value::Null);
            let tag = body
                .get("tag_name")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            if tag.is_empty() {
                Err(MineError::ExternalDependency {
                    detail: "GitHub releases/latest returned no tag_name".to_string(),
                })
            } else {
                Ok(tag)
            }
        }
        Err(ureq::Error::StatusCode(404)) => Err(MineError::ExternalDependency {
            detail: "no published release found (404). Publish a v* tag first.".to_string(),
        }),
        Err(e) => Err(MineError::ExternalDependency {
            detail: format!("GitHub releases/latest request failed: {e}"),
        }),
    }
}

/// Returns true if `latest_tag` is strictly newer than `current` by semver.
/// Tags may or may not have a leading `v`; both are tolerated.
pub fn is_newer(latest_tag: &str, current: &str) -> bool {
    let l = parse_semver(latest_tag);
    let c = parse_semver(current);
    match (l, c) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Parses a version string like `v0.1.0` or `0.1.0` into a 3-tuple.
fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let t = s.trim_start_matches('v');
    let parts: Vec<&str> = t.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}
