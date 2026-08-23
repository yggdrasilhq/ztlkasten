//! Host-local workspace configuration and read-only Obsidian vault discovery.
//!
//! The configured roots live with yggterm, never in a vault. Discovery only
//! reads directory structure; it does not create, rename, or annotate notes.

use crate::manifest::{Collection, Manifest, NodeShape, Order, Source};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "version")]
    pub version: u8,
    #[serde(default, rename = "master")]
    pub masters: Vec<PathBuf>,
}

fn version() -> u8 {
    1
}

pub fn path() -> Result<PathBuf> {
    if let Some(home) = std::env::var_os("YGGTERM_HOME") {
        return Ok(PathBuf::from(home).join("kasten/config.toml"));
    }
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".yggterm/kasten/config.toml"))
}

pub fn exists() -> bool {
    path().is_ok_and(|p| p.is_file())
}

pub fn load() -> Result<Config> {
    let file = path()?;
    let text =
        std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
    let config: Config =
        toml::from_str(&text).with_context(|| format!("parsing {}", file.display()))?;
    if config.masters.is_empty() {
        bail!("{} has no master folders", file.display());
    }
    Ok(config)
}

pub fn configure(paths: Vec<PathBuf>) -> Result<Config> {
    let mut masters = if exists() {
        load()?.masters
    } else {
        Vec::new()
    };
    for raw in paths {
        let expanded = expand_home(&raw)?;
        if !expanded.is_dir() {
            bail!("{} is not a directory", expanded.display());
        }
        let canonical = expanded.canonicalize()?;
        if !masters.contains(&canonical) {
            masters.push(canonical);
        }
    }
    if masters.is_empty() {
        bail!("at least one master folder is required");
    }
    let config = Config {
        version: 1,
        masters,
    };
    save(&config)?;
    Ok(config)
}

pub fn guided() -> Result<Config> {
    if !std::io::stdin().is_terminal() {
        bail!("Kasten has no master folders yet. Run `kasten init --master <folder>` in a terminal (repeat --master for more than one).");
    }
    println!("Kasten setup — vaults remain untouched; only ~/.yggterm is configured.");
    println!(
        "Enter one master folder per line. An Obsidian vault is a child containing .obsidian."
    );
    println!("Press Enter on an empty line when finished.");
    let mut paths = Vec::new();
    loop {
        print!(
            "Master folder{}: ",
            if paths.is_empty() { "" } else { " (or finish)" }
        );
        std::io::stdout().flush()?;
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        paths.push(PathBuf::from(line));
    }
    configure(paths)
}

fn save(config: &Config) -> Result<()> {
    let file = path()?;
    let parent = file.parent().context("invalid Kasten config path")?;
    std::fs::create_dir_all(parent)?;
    let text = toml::to_string_pretty(config)?;
    let temporary = file.with_extension("toml.new");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, &file)?;
    println!(
        "Kasten configured {} master folder(s) in {}",
        config.masters.len(),
        file.display()
    );
    Ok(())
}

fn expand_home(path: &Path) -> Result<PathBuf> {
    let text = path.to_string_lossy();
    if text == "~" || text.starts_with("~/") {
        let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
        return Ok(if text == "~" {
            home
        } else {
            home.join(&text[2..])
        });
    }
    Ok(path.to_path_buf())
}

pub fn manifest(config: &Config) -> Result<Manifest> {
    let mut vaults = Vec::new();
    for master in &config.masters {
        if master.join(".obsidian").is_dir() {
            vaults.push(master.clone());
        }
        let mut children: Vec<PathBuf> = std::fs::read_dir(master)
            .with_context(|| format!("reading master folder {}", master.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir() && p.join(".obsidian").is_dir())
            .collect();
        children.sort();
        vaults.extend(children);
    }
    vaults.sort();
    vaults.dedup();
    if vaults.is_empty() {
        bail!("no Obsidian vaults found under the configured master folders");
    }

    let mut collections = Vec::new();
    let mut ids = BTreeSet::new();
    let mut vault_ids = BTreeSet::new();
    for vault_path in &vaults {
        let vault = vault_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Vault")
            .to_string();
        let vault_base = safe_id(&vault);
        let mut vault_id = vault_base.clone();
        let mut vault_suffix = 2;
        while !vault_ids.insert(vault_id.clone()) {
            vault_id = format!("{vault_base}-{vault_suffix}");
            vault_suffix += 1;
        }
        let dirs = note_dirs(vault_path)?;
        for dir in dirs {
            let rel = dir.strip_prefix(vault_path).unwrap_or(Path::new(""));
            let rel_label = if rel.as_os_str().is_empty() {
                "Notes".to_string()
            } else {
                rel.display().to_string()
            };
            let base = safe_id(&format!("{vault}-{rel_label}"));
            let mut id = base.clone();
            let mut suffix = 2;
            while !ids.insert(id.clone()) {
                id = format!("{base}-{suffix}");
                suffix += 1;
            }
            collections.push(Collection {
                id,
                label: rel_label,
                vault: Some(vault.clone()),
                vault_id: Some(vault_id.clone()),
                path: dir,
                shape: NodeShape::Note,
                entry: None,
                title: Source::Heading,
                date: None,
                status: None,
                order: Order::Title,
                publish: false,
            });
        }
    }
    if collections.is_empty() {
        bail!("the discovered vaults contain no Markdown note folders");
    }
    Ok(Manifest {
        name: "Kasten".into(),
        root: config.masters[0].clone(),
        recent: 12,
        capture: None,
        collections,
    })
}

fn note_dirs(vault: &Path) -> Result<Vec<PathBuf>> {
    let mut candidates = vec![vault.to_path_buf()];
    candidates.extend(
        std::fs::read_dir(vault)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.is_dir()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| !n.starts_with('.'))
            }),
    );
    candidates.sort();
    let mut out = Vec::new();
    for dir in candidates {
        let has_notes = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).any(|e| {
            e.path().is_file()
                && matches!(
                    e.path().extension().and_then(|x| x.to_str()),
                    Some("md" | "markdown")
                )
        });
        if has_notes {
            out.push(dir);
        }
    }
    Ok(out)
}

fn safe_id(raw: &str) -> String {
    let mut out = String::new();
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_sibling_vaults_without_writing_to_them() {
        let root = std::env::temp_dir().join(format!("kasten-workspace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (vault, folder, note) in [
            ("north", "journal", "one.md"),
            ("south", "topics", "two.md"),
        ] {
            std::fs::create_dir_all(root.join(vault).join(".obsidian")).unwrap();
            std::fs::create_dir_all(root.join(vault).join(folder)).unwrap();
            std::fs::write(root.join(vault).join(folder).join(note), "# Invented\n").unwrap();
        }
        let m = manifest(&Config {
            version: 1,
            masters: vec![root.clone()],
        })
        .unwrap();
        assert_eq!(m.collections.len(), 2);
        assert_eq!(
            m.collections
                .iter()
                .filter_map(|c| c.vault.as_deref())
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );
        assert!(!root.join("north/kasten.toml").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
