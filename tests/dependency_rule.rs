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

fn upward_imports(dir: &str, forbidden: &[&str]) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(dir);
    let mut found = Vec::new();
    for file in rust_files(&root) {
        let source = fs::read_to_string(&file).expect("read source file");
        for (number, line) in source.lines().enumerate() {
            for path in forbidden {
                if line.contains(path) {
                    found.push(format!(
                        "{}:{}: {}",
                        file.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    found
}

#[test]
fn kits_never_name_a_flow_family_or_an_asset_module() {
    let found = upward_imports("kits", &["flows::", "tokens::"]);
    assert!(
        found.is_empty(),
        "upward imports in kits:\n{}",
        found.join("\n")
    );
}

#[test]
fn flows_never_name_an_asset_module() {
    let found = upward_imports("flows", &["tokens::"]);
    assert!(
        found.is_empty(),
        "upward imports in flows:\n{}",
        found.join("\n")
    );
}
