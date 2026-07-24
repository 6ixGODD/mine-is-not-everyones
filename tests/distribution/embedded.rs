// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Embedded-skills tests: verify the build-time-embedded Skill payload matches
//! the authoritative root `skills/` directory byte-for-byte, with no missing or
//! extra files.

use super::common::*;
use mine::infrastructure::embedded_skills;

#[test]
fn embedded_skills_match_root_byte_for_byte() {
    // Every file in skills/ must be embedded, and every embedded entry must
    // match its root file byte-for-byte.
    let root_files = rel_files(&skills_root());
    let embedded_paths: std::collections::HashSet<String> = embedded_skills::paths()
        .into_iter()
        .map(|p| p.strip_prefix("skills/").unwrap().to_string())
        .collect();
    assert_eq!(
        root_files, embedded_paths,
        "embedded file set must exactly match root skills/ file set"
    );

    for rel in &root_files {
        let embedded = embedded_skills::get(&format!("skills/{rel}"))
            .unwrap_or_else(|| panic!("embedded skills missing skills/{rel}"));
        let root_content = std::fs::read_to_string(skills_root().join(rel)).unwrap();
        assert_eq!(
            embedded, root_content,
            "embedded skills/{rel} must be byte-for-byte identical to root"
        );
    }
}

#[test]
fn embedded_skills_cannot_omit_a_new_skill_file() {
    // If a new file appears in skills/ but is not added to EMBEDDED_SKILL_FILES,
    // this test fails - catching build-time drift.
    let root_files = rel_files(&skills_root());
    let embedded: std::collections::HashSet<String> = embedded_skills::paths()
        .into_iter()
        .map(|p| p.strip_prefix("skills/").unwrap().to_string())
        .collect();
    for rel in &root_files {
        assert!(
            embedded.contains(rel),
            "root file {rel} is not embedded - add it to EMBEDDED_SKILL_FILES"
        );
    }
}

#[test]
fn embedded_skills_lookup_returns_content() {
    let content = embedded_skills::get("skills/mine-arch/SKILL.md");
    assert!(
        content.is_some(),
        "lookup must find embedded mine-arch/SKILL.md"
    );
    let body = content.unwrap();
    assert!(
        body.contains("MINE Architecture"),
        "embedded mine-arch/SKILL.md must contain its title"
    );
}

#[test]
fn embedded_skills_lookup_returns_none_for_unknown() {
    assert!(
        embedded_skills::get("skills/nonexistent/SKILL.md").is_none(),
        "lookup must return None for an unknown path"
    );
}

#[test]
fn all_five_skill_directories_are_embedded() {
    let dirs: std::collections::HashSet<&str> = embedded_skills::paths()
        .into_iter()
        .filter_map(|p| p.strip_prefix("skills/").and_then(|s| s.split('/').next()))
        .collect();
    for s in FIVE_SKILLS {
        assert!(dirs.contains(s), "Skill directory {s} must be embedded");
    }
}

#[test]
fn embedded_paths_include_all_reference_files() {
    // The embedded payload must include reference files, not just SKILL.md.
    let paths = embedded_skills::paths();
    assert!(
        paths.iter().any(|p| p.contains("/references/")),
        "embedded payload must include reference files"
    );
}

#[test]
fn embedded_content_is_statically_borrowed() {
    // The embedded content is 'static (compiled into the binary), so it does
    // not require a live checkout at runtime.
    let content: &'static str = embedded_skills::get("skills/mine-sync/SKILL.md").unwrap();
    assert!(!content.is_empty());
    // Prove it is 'static by sending it across a thread boundary.
    let handle = std::thread::spawn(move || content.len());
    assert!(handle.join().unwrap() > 0);
}
