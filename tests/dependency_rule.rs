//! The dependency rule of the design: `tokens` may use `flows` and `kits`,
//! `flows` may use `kits`, and nothing imports upward.
//!
//! A kit has no reason to name the `flows` or `tokens` module, and a flow
//! family has no reason to name `tokens`, so any such path in their source
//! is an upward import. The scan reads every source line, so a fully
//! qualified path in a function body counts the same as a `use` line.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

/// The first forbidden path this source line names, if any. A comment line
/// never counts, and a path must start at a word boundary, so `my_tokens::`
/// and `workflows::` do not match.
fn upward_import<'a>(line: &str, forbidden: &[&'a str]) -> Option<&'a str> {
    if line.trim_start().starts_with("//") {
        return None;
    }
    forbidden.iter().copied().find(|path| {
        line.match_indices(path).any(|(at, _)| {
            line[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
        })
    })
}

fn upward_imports(dir: &str, forbidden: &[&str]) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(dir);
    let mut found = Vec::new();
    for file in rust_files(&root) {
        let source = fs::read_to_string(&file).expect("read source file");
        for (number, line) in source.lines().enumerate() {
            if upward_import(line, forbidden).is_some() {
                found.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    found
}

#[test]
fn kits_never_name_a_flow_family_or_an_asset_module() {
    let found = upward_imports(
        "kits",
        &["flows::", "tokens::", "crate::flows", "crate::tokens"],
    );
    assert!(
        found.is_empty(),
        "upward imports in kits:\n{}",
        found.join("\n")
    );
}

#[test]
fn flows_never_name_an_asset_module() {
    let found = upward_imports("flows", &["tokens::", "crate::tokens"]);
    assert!(
        found.is_empty(),
        "upward imports in flows:\n{}",
        found.join("\n")
    );
}

#[cfg(test)]
mod matcher {
    use super::upward_import;

    const KITS: &[&str] = &["flows::", "tokens::", "crate::flows", "crate::tokens"];

    #[test]
    fn a_fully_qualified_path_in_a_body_hits() {
        let line = "    let n = crate::flows::family::shared();";
        assert!(upward_import(line, KITS).is_some());
    }

    #[test]
    fn an_aliased_import_hits() {
        assert_eq!(
            upward_import("use crate::flows as f;", KITS),
            Some("crate::flows")
        );
    }

    #[test]
    fn a_parent_path_through_super_does_not_hit() {
        assert_eq!(
            upward_import(
                "pub fn encode(s: &str) -> bool { super::validate_address(s) }",
                KITS
            ),
            None
        );
    }

    #[test]
    fn a_comment_line_does_not_hit() {
        assert_eq!(
            upward_import(
                "//! An integrator reaches `tokens::beth::mint` for this.",
                KITS
            ),
            None
        );
        assert_eq!(
            upward_import("    // see flows::canton_bridge_v1", KITS),
            None
        );
    }

    #[test]
    fn a_longer_identifier_does_not_hit() {
        assert_eq!(upward_import("let x = my_tokens::thing();", KITS), None);
        assert_eq!(upward_import("let y = workflows::run();", KITS), None);
    }
}
