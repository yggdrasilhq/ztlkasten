//! Capture — the first of the two hot paths, and the one the design value
//! actually ranks.
//!
//! **The whole budget: one command, zero decisions.** A thought arrives, it
//! lands in today's entry, and the writer is not asked which file, which title,
//! which folder or which tag. Every question this code could ask the writer has
//! been moved into the manifest and answered once, or derived from what the
//! collection already declares.
//!
//! ⛔ NO SECOND ENCODING OF "HOW A JOURNAL ENTRY IS NAMED". The target file name
//! comes from the collection's own declared date source rather than from a
//! capture-specific setting. A corpus that said `date = "filename"` has already
//! said how its entries are named, and a second place to say it is a second
//! place to disagree.

use crate::manifest::{Manifest, Source};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Captured {
    pub path: PathBuf,
    /// True when this capture started the day's entry rather than adding to it.
    pub created: bool,
}

/// Where a capture lands, resolved from the manifest alone.
///
/// Separated from the write so a caller can show the destination without
/// creating anything — and so the test suite can assert the resolution without
/// touching a disk.
pub fn target(manifest: &Manifest, today: &str) -> Result<PathBuf> {
    let Some(id) = &manifest.capture else {
        bail!(
            "this corpus declares no capture target — add [capture] with a collection to journal into"
        );
    };
    let Some(collection) = manifest.collection(id) else {
        bail!("[capture] names collection {id:?}, which this corpus does not declare");
    };
    Ok(collection.path.join(format!("{today}.md")))
}

/// Append a thought to today's entry, creating it if this is the first one.
///
/// ⚠ The collection's directory is created if absent. A capture that failed
/// because a folder did not exist yet would be exactly the bureaucratic
/// resistance this program is judged on — the writer would have to stop, read
/// an error, and make a directory before recording a thought.
pub fn write(manifest: &Manifest, today: &str, at: &str, text: &str) -> Result<Captured> {
    let text = text.trim();
    if text.is_empty() {
        bail!("nothing to capture — an empty entry is a file to clean up later, not a thought");
    }

    let path = target(manifest, today)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let created = !path.exists();
    let mut body = String::new();
    if created {
        // The heading is the date, which is what a note collection reads its
        // title from. The entry names itself; the writer does not title it.
        body.push_str(&format!("# {today}\n"));
    }
    // A time marker before every thought, including the first. It costs the
    // writer nothing — they never type it — and it is the only way to recover
    // the order of a day's thinking months later. The flow budget prices rules
    // in DECISIONS and keystrokes on the hot path; this adds neither.
    body.push_str(&format!("\n## {at}\n\n{text}\n"));

    let mut existing = if created {
        String::new()
    } else {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    };
    existing.push_str(&body);
    std::fs::write(&path, existing).with_context(|| format!("writing {}", path.display()))?;

    Ok(Captured { path, created })
}

/// What a corpus must satisfy for capture to be possible, checked at manifest
/// load so a misconfiguration is a named error rather than a thought that lands
/// somewhere unfindable.
pub fn validate(manifest: &Manifest) -> Result<()> {
    let Some(id) = &manifest.capture else {
        return Ok(());
    };
    let Some(collection) = manifest.collection(id) else {
        bail!("[capture] names collection {id:?}, which this corpus does not declare");
    };
    if collection.shape != crate::manifest::NodeShape::Note {
        bail!(
            "[capture] names {id:?}, which holds records — a captured thought is prose, and \
             writing it into a facts collection would put it where nothing reads it"
        );
    }
    if collection.date != Some(Source::Filename) {
        bail!(
            "[capture] names {id:?}, whose date does not come from the filename — capture writes \
             a dated file, so the collection has to be able to read a date back out of one"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Corpus;

    const MANIFEST: &str = "[corpus]\nname = \"Example\"\n\n[capture]\ncollection = \"journal\"\n\n\
         [[collection]]\nid = \"journal\"\nnode = \"note\"\ndate = \"filename\"\norder = \"date-desc\"\n";

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kasten-cap-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn manifest(dir: &PathBuf) -> Manifest {
        Manifest::parse(MANIFEST, dir).unwrap()
    }

    /// The budget, asserted: one call, no prompts, and the collection's own
    /// directory did not have to exist first.
    #[test]
    fn a_first_thought_creates_the_day_and_is_readable_immediately() {
        let dir = scratch("first");
        let m = manifest(&dir);
        let out = write(&m, "2031-05-04", "09:12", "the culvert ran clear").unwrap();
        assert!(out.created);

        let corpus = Corpus::load(manifest(&dir)).unwrap();
        assert_eq!(corpus.count("journal"), 1);
        let node = corpus.node("journal/2031-05-04").unwrap();
        // The entry titles itself from its own date, so the writer never did.
        assert_eq!(node.title, "2031-05-04");
        assert_eq!(node.date.as_deref(), Some("2031-05-04"));
        assert!(node.body.as_deref().unwrap().contains("the culvert ran clear"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_second_thought_joins_the_same_day_rather_than_starting_a_new_entry() {
        let dir = scratch("second");
        let m = manifest(&dir);
        write(&m, "2031-05-04", "09:12", "first thought").unwrap();
        let out = write(&m, "2031-05-04", "14:40", "second thought").unwrap();
        assert!(!out.created);

        let corpus = Corpus::load(manifest(&dir)).unwrap();
        assert_eq!(corpus.count("journal"), 1, "a day is one entry, not one per thought");
        let body = corpus.node("journal/2031-05-04").unwrap().body.clone().unwrap();
        assert!(body.contains("first thought") && body.contains("second thought"));
        // One heading for the day, one time marker per thought.
        assert_eq!(body.matches("# 2031-05-04").count(), 1);
        assert!(body.contains("## 09:12") && body.contains("## 14:40"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_link_typed_mid_sentence_is_indexed_with_no_second_gesture() {
        // Retrieval is the other hot path, and it must not cost the writer a
        // separate declaration — the link they typed IS the index entry.
        let dir = scratch("links");
        let m = manifest(&dir);
        write(&m, "2031-05-04", "09:12", "saw it again at the [[culvert]]").unwrap();
        let corpus = Corpus::load(manifest(&dir)).unwrap();
        assert_eq!(corpus.node("journal/2031-05-04").unwrap().links, vec!["culvert"]);
        // And an unwritten target becomes a to-write list the writer produced
        // by writing, at a cost of zero extra keystrokes.
        assert_eq!(corpus.unresolved(), vec!["culvert"]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_empty_thought_is_refused_rather_than_filed() {
        let dir = scratch("empty");
        let m = manifest(&dir);
        for blank in ["", "   ", "\n\t "] {
            let err = write(&m, "2031-05-04", "09:12", blank).unwrap_err().to_string();
            assert!(err.contains("nothing to capture"), "{err}");
        }
        assert!(!dir.join("journal/2031-05-04.md").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_corpus_with_no_capture_target_says_so_instead_of_guessing_one() {
        let dir = scratch("nocap");
        let text = "[corpus]\nname = \"E\"\n\n[[collection]]\nid = \"journal\"\nnode = \"note\"\n";
        let m = Manifest::parse(text, &dir).unwrap();
        let err = write(&m, "2031-05-04", "09:12", "thought").unwrap_err().to_string();
        assert!(err.contains("no capture target"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Both refusals are at LOAD, so a corpus cannot be configured to swallow
    /// thoughts into a collection that will never show them.
    #[test]
    fn capture_into_a_collection_that_cannot_hold_a_dated_note_is_refused_at_load() {
        let dir = scratch("bad");

        let records = "[corpus]\nname = \"E\"\n\n[capture]\ncollection = \"items\"\n\n\
            [[collection]]\nid = \"items\"\nnode = \"record\"\n";
        let err = Manifest::parse(records, &dir).unwrap_err().to_string();
        assert!(err.contains("holds records"), "{err}");

        let undated = "[corpus]\nname = \"E\"\n\n[capture]\ncollection = \"journal\"\n\n\
            [[collection]]\nid = \"journal\"\nnode = \"note\"\n";
        let err = Manifest::parse(undated, &dir).unwrap_err().to_string();
        assert!(err.contains("date does not come from the filename"), "{err}");

        let missing = "[corpus]\nname = \"E\"\n\n[capture]\ncollection = \"nope\"\n\n\
            [[collection]]\nid = \"journal\"\nnode = \"note\"\ndate = \"filename\"\n";
        let err = Manifest::parse(missing, &dir).unwrap_err().to_string();
        assert!(err.contains("does not declare"), "{err}");

        // POSITIVE CONTROL: the shape it is meant to accept still loads, or the
        // three refusals above prove only that the validator says no to things.
        assert!(Manifest::parse(MANIFEST, &dir).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_target_can_be_shown_without_creating_anything() {
        let dir = scratch("target");
        let m = manifest(&dir);
        let path = target(&m, "2031-05-04").unwrap();
        assert!(path.ends_with("journal/2031-05-04.md"));
        assert!(!path.exists(), "resolving a destination must not create it");
        std::fs::remove_dir_all(&dir).ok();
    }
}
