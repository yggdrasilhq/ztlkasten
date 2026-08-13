//! The LAUNCHER MANIFEST — how the host's menus learn this app exists.
//!
//! Written on the app's own host on every run, which repairs the recorded
//! binary path after an upgrade. The host scans the directory and drops
//! manifests whose binary is gone; that is the entire uninstall story.
//!
//! ⚠ Not to be confused with `kasten.toml`, which is a CORPUS manifest. This
//! one describes the program to the desktop; that one describes a corpus to the
//! program. Two different files answering two different questions, and the
//! naming collision is worth the sentence it costs to say so.

use anyhow::Result;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn manifest_value(binary: &Path) -> Value {
    json!({
        "name": "kasten",
        "label": "Kasten",
        "icon": "🗃\u{fe0e}",
        "binary": binary.to_string_lossy(),
        "verbs": [
            // No corpus argument: the app resolves one from the working
            // directory, which is where a corpus is opened from.
            { "id": "open", "label": "Open Kasten", "args": ["serve"] },
        ],
    })
}

fn write_to(apps_dir: &Path, binary: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(apps_dir)?;
    let path = apps_dir.join("kasten.json");
    std::fs::write(&path, serde_json::to_string_pretty(&manifest_value(binary))?)?;
    Ok(path)
}

/// Best-effort on every run. A desktop that cannot be told about the app is not
/// a reason to refuse to show the corpus.
pub fn write_best_effort() {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let Ok(binary) = std::env::current_exe() else {
        return;
    };
    let apps = Path::new(&home).join(".yggterm").join("apps");
    let _ = write_to(&apps, &binary);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_names_the_binary_absolutely_and_declares_one_verb() {
        let value = manifest_value(Path::new("/usr/local/bin/kasten"));
        assert_eq!(value["name"], "kasten");
        assert!(value["binary"].as_str().unwrap().starts_with('/'));
        assert_eq!(value["verbs"].as_array().unwrap().len(), 1);
    }
}
