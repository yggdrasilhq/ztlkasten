//! `kasten init` — propose a manifest by looking at what a corpus actually is.
//!
//! Writing a manifest by hand is a chore assembled from primitives: list the
//! directories, work out whether each holds prose or facts, guess where the
//! titles are, get one wrong, notice weeks later. A verb does it the same way
//! every time.
//!
//! ⛔ IT PROPOSES, IT DOES NOT DECIDE. The output goes to stdout for a human to
//! read and edit, and an existing manifest is never overwritten. A generator
//! that silently replaced a hand-tuned file would make the manifest untrustworthy
//! exactly where it matters most — on a corpus nobody wants to re-check.
//!
//! ⛔ AND IT NEVER GUESSES A SOURCE THAT IS NOT THERE. A date source is emitted
//! only if dated filenames or a date key were actually seen. An inferred field
//! that finds nothing at runtime sorts a listing by nothing and is never
//! questioned, which is worse than an absent one the reader can see is absent.

use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Directories that are never a collection. Build output, version control and
/// the corpus's own tooling — none of them hold nodes, and proposing them makes
/// a human edit the output before they can trust any of it.
const SKIP: &[&str] = &[
    ".git", "bin", "target", "node_modules", "scripts", "venv", ".venv", "__pycache__", "backups",
    "tmp", "cache", "assets", "vendor",
];

struct Survey {
    id: String,
    notes: usize,
    records: usize,
    dated_names: usize,
    heading_titles: usize,
    fact_keys: BTreeSet<String>,
    /// For directory nodes: what the facts file inside them is called, and how
    /// often. Corpora name it after the node kind rather than using one
    /// universal name, so it is observed rather than assumed.
    entry_names: BTreeMap<String, usize>,
}

impl Survey {
    /// The name most of this collection's directory nodes agree on. Ties are
    /// broken by name so two runs on one corpus cannot disagree.
    fn entry_name(&self) -> Option<String> {
        self.entry_names
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(name, _)| name.clone())
    }

    fn total(&self) -> usize {
        self.notes + self.records
    }

    /// The first of a set of likely title keys that this collection's facts
    /// actually carry. Order is preference, not alphabetical.
    fn fact_key(&self, candidates: &[&str]) -> Option<String> {
        candidates
            .iter()
            .find(|k| self.fact_keys.contains(**k))
            .map(|k| k.to_string())
    }
}

pub fn propose(root: &Path) -> Result<String> {
    let manifest = root.join("kasten.toml");
    if manifest.exists() {
        bail!(
            "{} already exists — read it, edit it, or move it aside; this verb will not overwrite a manifest",
            manifest.display()
        );
    }
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.starts_with('.') && !SKIP.contains(&n))
        })
        .collect();
    dirs.sort();

    let surveys: Vec<Survey> = dirs.iter().filter_map(|d| survey(d).ok()).collect();
    let populated: Vec<&Survey> = surveys.iter().filter(|s| s.total() > 0).collect();

    if populated.is_empty() {
        bail!(
            "no collection-shaped directories under {} — every candidate was empty of prose and facts. \
             Name the root with --corpus, or write the manifest by hand",
            root.display()
        );
    }

    let name = root
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Corpus".to_string());

    let mut out = String::new();
    out.push_str(
        "# Proposed by `kasten init` — READ IT BEFORE YOU KEEP IT.\n\
         #\n\
         # Everything here was inferred from what is on disk right now, so it\n\
         # describes the corpus as it happens to be rather than as it is meant\n\
         # to be. Labels in particular are directory names and are almost\n\
         # certainly not what you would call these things out loud.\n\n",
    );
    out.push_str(&format!("[corpus]\nname = {name:?}\n\n[overview]\nrecent = 8\n"));

    let skipped: Vec<&Survey> = surveys.iter().filter(|s| s.total() == 0).collect();

    for s in &populated {
        // The shape is whichever the collection has more of. A mixed directory
        // is reported rather than silently resolved, because the minority is
        // then invisible to the reader and there is no way to notice.
        let record = s.records > s.notes;
        out.push_str(&format!("\n[[collection]]\nid = {:?}\n", s.id));
        out.push_str(&format!("label = {:?}\n", pretty(&s.id)));
        out.push_str(&format!(
            "node = \"{}\"\n",
            if record { "record" } else { "note" }
        ));
        // Only when it differs from the default, so a manifest carries a line
        // per thing that is actually unusual about its corpus.
        if let Some(entry) = s.entry_name().filter(|e| e != "index.toml") {
            out.push_str(&format!("entry = {entry:?}\n"));
        }

        let title = if record {
            s.fact_key(&["name", "title", "label"])
                .map(|k| format!("facts:{k}"))
        } else if s.heading_titles * 2 >= s.notes.max(1) {
            Some("heading".to_string())
        } else {
            None
        };
        if let Some(title) = &title {
            out.push_str(&format!("title = {title:?}\n"));
        }

        // A date source only when one was actually observed.
        let date = if s.dated_names * 2 >= s.total() && s.dated_names > 0 {
            Some("filename".to_string())
        } else {
            s.fact_key(&["date", "opened", "filed", "started"])
                .map(|k| format!("facts:{k}"))
        };
        if let Some(date) = &date {
            out.push_str(&format!("date = {date:?}\n"));
            out.push_str("order = \"date-desc\"\n");
        }
        if let Some(status) = s.fact_key(&["status", "state", "stage"]) {
            out.push_str(&format!("status = \"facts:{status}\"\n"));
        }

        out.push_str(&format!(
            "# {} node{} seen ({} prose, {} record)\n",
            s.total(),
            if s.total() == 1 { "" } else { "s" },
            s.notes,
            s.records,
        ));
        if s.notes > 0 && s.records > 0 {
            out.push_str(
                "# ⚠ MIXED: both shapes are present here and only one can be declared.\n\
                 #   Whichever you do not pick becomes invisible — split the directory\n\
                 #   or accept the loss knowingly.\n",
            );
        }
    }

    if !skipped.is_empty() {
        out.push_str("\n# Directories seen and NOT proposed, because they held no prose or\n# facts files. Named so their absence is a fact rather than an oversight:\n");
        for s in skipped {
            out.push_str(&format!("#   {}\n", s.id));
        }
    }

    Ok(out)
}

fn survey(dir: &Path) -> Result<Survey> {
    let id = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let mut s = Survey {
        id,
        notes: 0,
        records: 0,
        dated_names: 0,
        heading_titles: 0,
        fact_keys: BTreeSet::new(),
        entry_names: BTreeMap::new(),
    };

    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();

    for entry in entries {
        let stem = entry
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if stem.is_empty() || stem.starts_with('.') {
            continue;
        }
        let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("");

        if entry.is_dir() {
            // A directory node declares itself by holding exactly ONE top-level
            // facts file. Two would make "which one is the node" a guess, and a
            // guess here is invisible: the wrong file parses fine and the node
            // shows the wrong title forever.
            let mut tomls: Vec<String> = std::fs::read_dir(&entry)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().and_then(|x| x.to_str()) == Some("toml"))
                .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
                .collect();
            tomls.sort();
            if let [only] = tomls.as_slice() {
                s.records += 1;
                *s.entry_names.entry(only.clone()).or_default() += 1;
                collect_keys(&entry.join(only), &mut s);
                if dated(&stem) {
                    s.dated_names += 1;
                }
            }
            continue;
        }

        match ext {
            "toml" => {
                s.records += 1;
                collect_keys(&entry, &mut s);
                if dated(&stem) {
                    s.dated_names += 1;
                }
            }
            "md" | "markdown" => {
                // A prose file sitting beside a facts file of the same stem is
                // that record's body, not a node of its own. Counting it twice
                // is how a corpus of 60 records reports 120 nodes.
                if entry.with_extension("toml").is_file() {
                    continue;
                }
                s.notes += 1;
                if dated(&stem) {
                    s.dated_names += 1;
                }
                if let Ok(text) = std::fs::read_to_string(&entry) {
                    if text.lines().any(|l| l.starts_with("# ")) {
                        s.heading_titles += 1;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(s)
}

/// Top-level scalar keys only. A nested table's keys are not proposed, because
/// a dotted guess that happens to resolve is the kind of inference nobody
/// re-reads.
fn collect_keys(path: &Path, s: &mut Survey) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(toml::Value::Table(table)) = toml::from_str::<toml::Value>(&text) else {
        return;
    };
    for (key, value) in table {
        if !matches!(value, toml::Value::Table(_)) {
            s.fact_keys.insert(key);
        }
    }
}

fn dated(stem: &str) -> bool {
    let b = stem.as_bytes();
    b.len() >= 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
}

fn pretty(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for (i, word) in id.split(['-', '_']).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn it_refuses_to_overwrite_a_manifest_that_exists() {
        let err = propose(&fixture("fieldbook")).unwrap_err().to_string();
        assert!(err.contains("will not overwrite"), "{err}");
    }

    /// The real test of a generator: what it proposes must LOAD, and must
    /// describe the same corpus the hand-written manifest does.
    #[test]
    fn what_it_proposes_parses_and_finds_the_same_collections() {
        let dir = tempdir("propose-note");
        for (path, body) in [
            ("journal/2031-01-02-one.md", "# One\n\nprose\n"),
            ("journal/2031-01-09-two.md", "# Two\n\nprose\n"),
            ("indices/topics.md", "# Topics\n\nprose\n"),
        ] {
            write(&dir, path, body);
        }

        let proposed = propose(&dir).unwrap();
        let manifest = crate::manifest::Manifest::parse(&proposed, &dir).unwrap();
        let ids: Vec<&str> = manifest.collections.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["indices", "journal"]);

        let journal = manifest.collection("journal").unwrap();
        assert_eq!(journal.title, crate::manifest::Source::Heading);
        // Dated filenames were actually there, so a date source is proposed.
        assert_eq!(journal.date, Some(crate::manifest::Source::Filename));

        // And they were NOT there for indices, so none is invented.
        assert_eq!(manifest.collection("indices").unwrap().date, None);

        let corpus = crate::corpus::Corpus::load(manifest).unwrap();
        assert_eq!(corpus.count("journal"), 2);
        assert_eq!(corpus.count("indices"), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_record_collection_is_recognised_and_its_prose_is_not_double_counted() {
        let dir = tempdir("propose-record");
        write(&dir, "items/alpha.toml", "name = \"Alpha\"\nstatus = \"durable\"\n");
        write(&dir, "items/alpha.md", "the body of alpha\n");
        write(&dir, "items/beta.toml", "name = \"Beta\"\nstatus = \"transient\"\n");

        let proposed = propose(&dir).unwrap();
        assert!(proposed.contains("node = \"record\""), "{proposed}");
        assert!(proposed.contains("title = \"facts:name\""), "{proposed}");
        assert!(proposed.contains("status = \"facts:status\""), "{proposed}");
        // Two nodes, not three: alpha.md is alpha's body, not a node.
        assert!(proposed.contains("# 2 nodes seen"), "{proposed}");

        let manifest = crate::manifest::Manifest::parse(&proposed, &dir).unwrap();
        assert_eq!(crate::corpus::Corpus::load(manifest).unwrap().count("items"), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The gap that real corpora exposed: a directory node names its facts file
    /// after the node KIND, not `index.toml`. Detected, declared, and then it
    /// has to actually find the nodes — a proposal that names the file and
    /// still reads zero nodes would look right and be useless.
    #[test]
    fn a_directory_node_whose_facts_file_is_named_after_its_kind_is_found() {
        let dir = tempdir("propose-entry");
        write(&dir, "matters/first-thing/matter.toml", "name = \"First\"\nstatus = \"durable\"\n");
        write(&dir, "matters/first-thing/matter.md", "the body\n");
        write(&dir, "matters/second-thing/matter.toml", "name = \"Second\"\n");

        let proposed = propose(&dir).unwrap();
        assert!(proposed.contains("entry = \"matter.toml\""), "{proposed}");

        let manifest = crate::manifest::Manifest::parse(&proposed, &dir).unwrap();
        let corpus = crate::corpus::Corpus::load(manifest).unwrap();
        assert_eq!(corpus.count("matters"), 2);
        let first = corpus.node("matters/first-thing").unwrap();
        assert_eq!(first.title, "First");
        // The prose beside the facts file is the node's body, and it is found
        // by the facts file's stem rather than by a fixed `index.md`.
        assert!(first.body.is_some(), "the sibling prose was not picked up");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_default_entry_name_is_not_written_out() {
        // A manifest should carry a line per thing that is unusual about its
        // corpus, not a line per field that exists.
        let dir = tempdir("propose-default-entry");
        write(&dir, "items/one/index.toml", "name = \"One\"\n");
        let proposed = propose(&dir).unwrap();
        assert!(!proposed.contains("entry ="), "{proposed}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_directory_holding_two_facts_files_is_not_treated_as_a_node() {
        // Which one would be the node is a guess, and a wrong guess parses fine
        // and shows the wrong title forever.
        let dir = tempdir("propose-ambiguous");
        write(&dir, "things/one/matter.toml", "name = \"One\"\n");
        write(&dir, "things/one/other.toml", "name = \"Other\"\n");
        write(&dir, "things/two/matter.toml", "name = \"Two\"\n");
        let proposed = propose(&dir).unwrap();
        assert!(proposed.contains("# 1 node seen"), "{proposed}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_mixed_directory_is_flagged_rather_than_quietly_resolved() {
        let dir = tempdir("propose-mixed");
        write(&dir, "things/a.toml", "name = \"A\"\n");
        write(&dir, "things/loose.md", "# Loose\n");
        let proposed = propose(&dir).unwrap();
        assert!(proposed.contains("MIXED"), "{proposed}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_directory_with_nothing_in_it_is_named_rather_than_dropped() {
        let dir = tempdir("propose-empty");
        write(&dir, "real/a.md", "# A\n");
        std::fs::create_dir_all(dir.join("logs")).unwrap();
        let proposed = propose(&dir).unwrap();
        assert!(proposed.contains("NOT proposed"), "{proposed}");
        assert!(proposed.contains("#   logs"), "{proposed}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_corpus_with_no_node_bearing_directory_refuses_instead_of_emitting_an_empty_manifest() {
        let dir = tempdir("propose-nothing");
        std::fs::create_dir_all(dir.join("logs")).unwrap();
        let err = propose(&dir).unwrap_err().to_string();
        assert!(err.contains("no collection-shaped"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kasten-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
}
