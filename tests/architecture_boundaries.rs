use std::fs;
use std::path::{Path, PathBuf};

fn rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn rel(path: &Path) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    rel.replace('\\', "/")
}

#[test]
fn treemap_module_is_pure() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/treemap");
    let mut violations = Vec::new();

    for file in rs_files(&root) {
        let content = fs::read_to_string(&file).unwrap_or_default();
        for forbidden in ["crate::ui", "crate::system", "ratatui"] {
            if content.contains(forbidden) {
                violations.push(format!(
                    "{} imports forbidden dependency `{}`",
                    rel(&file),
                    forbidden
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Treemap layering violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_module_does_not_import_platform_extensions_directly() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut violations = Vec::new();

    for file in rs_files(&root) {
        let content = fs::read_to_string(&file).unwrap_or_default();
        if content.contains("crate::system::platform") {
            violations.push(format!(
                "{} imports `crate::system::platform` directly",
                rel(&file)
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "UI/platform boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn target_os_cfg_is_scoped_to_system_platform_or_kill() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file in rs_files(&root) {
        let content = fs::read_to_string(&file).unwrap_or_default();
        if !content.contains("target_os") {
            continue;
        }

        let rel_path = rel(&file);
        let allowed =
            rel_path.starts_with("src/system/platform/") || rel_path == "src/system/kill.rs";
        if !allowed {
            violations.push(format!(
                "{} contains `target_os` cfg but is outside allowed boundary",
                rel_path
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Unexpected target_os cfg usage:\n{}",
        violations.join("\n")
    );
}
