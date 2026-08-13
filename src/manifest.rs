//! `kasten.toml` — the ONE contract between this engine and any corpus.
//!
//! The engine ships in public and must never learn a corpus's vocabulary. A
//! corpus declares its own: what its collections are called, what shape its
//! nodes have, and where a title, a date and a status are read from. Everything
//! downstream of this file is generic.
//!
//! ⛔ Nothing here may grow a default that names a real collection. A default
//! that happens to match one corpus is that corpus's vocabulary smuggled into
//! the engine, and the next corpus inherits a wrong answer that looks right.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Where a field's value is read from. `Heading` and `Filename` are the note
/// corpus's answers; `Facts` is the record corpus's. A corpus picks per field,
/// so the two shapes are not two code paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The first `# ` heading of the prose body.
    Heading,
    /// A key in the prose body's YAML frontmatter.
    Frontmatter(String),
    /// A key in the node's facts file. Dotted keys walk nested tables.
    Facts(String),
    /// A leading `YYYY-MM-DD` in the file name.
    Filename,
    /// The slug itself, with separators turned back into spaces.
    Slug,
    /// Filesystem modification time, as seconds since the epoch.
    Mtime,
}

impl Source {
    fn parse(raw: &str, field: &str) -> Result<Self> {
        let source = match raw {
            "heading" => Source::Heading,
            "filename" => Source::Filename,
            "slug" => Source::Slug,
            "mtime" => Source::Mtime,
            other => match other.split_once(':') {
                Some(("frontmatter", key)) if !key.is_empty() => {
                    Source::Frontmatter(key.to_string())
                }
                Some(("facts", key)) if !key.is_empty() => Source::Facts(key.to_string()),
                _ => bail!(
                    "{field}: unknown source {other:?} — expected heading, filename, slug, \
                     mtime, frontmatter:<key> or facts:<key>"
                ),
            },
        };
        Ok(source)
    }
}

/// What a node is made of, physically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    /// One prose file per node. The slug is the file stem.
    Note,
    /// A facts file plus optional prose of the same stem, or a directory
    /// holding `index.toml`. Both forms occur in real corpora.
    Record,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    DateDesc,
    DateAsc,
    Title,
    Slug,
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub shape: NodeShape,
    /// For a DIRECTORY node, the file inside it that holds the facts.
    ///
    /// Real corpora name this after the node kind — `matter.toml` inside a
    /// matter's directory — rather than using one universal name. That is a
    /// vocabulary, so it is declared here rather than guessed: a hard-coded
    /// list of likely filenames would work on the corpus it was written
    /// against and silently see nothing on the next one.
    pub entry: String,
    pub title: Source,
    pub date: Option<Source>,
    pub status: Option<Source>,
    pub order: Order,
    /// `false` ⇒ this collection may never enter a publication path. Declared,
    /// not inferred from where its directory sits.
    pub publish: bool,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub root: PathBuf,
    pub recent: usize,
    pub collections: Vec<Collection>,
}

// ---------------------------------------------------------------------------
// The wire form. Kept separate from the resolved form above so that an
// unparseable source is an error at LOAD time, naming the field — not a silent
// fallback discovered later by a reader wondering why every title is a slug.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RawFile {
    corpus: RawCorpus,
    #[serde(default)]
    overview: RawOverview,
    #[serde(default)]
    collection: Vec<RawCollection>,
}

#[derive(Deserialize)]
struct RawCorpus {
    name: String,
    root: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawOverview {
    recent: Option<usize>,
}

#[derive(Deserialize)]
struct RawCollection {
    id: String,
    label: Option<String>,
    path: Option<String>,
    node: String,
    entry: Option<String>,
    title: Option<String>,
    date: Option<String>,
    status: Option<String>,
    order: Option<String>,
    publish: Option<bool>,
}

const DEFAULT_RECENT: usize = 8;

impl Manifest {
    pub fn load(manifest_path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?;
        let dir = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::parse(&text, &dir)
            .with_context(|| format!("in {}", manifest_path.display()))
    }

    pub fn parse(text: &str, dir: &Path) -> Result<Self> {
        let raw: RawFile = toml::from_str(text).context("parsing kasten.toml")?;

        let root = match raw.corpus.root {
            Some(r) => dir.join(r),
            None => dir.to_path_buf(),
        };

        if raw.collection.is_empty() {
            bail!("no [[collection]] declared — a corpus with no collections has nothing to show");
        }

        let mut collections = Vec::with_capacity(raw.collection.len());
        for c in raw.collection {
            let id = c.id;
            if id.is_empty() {
                bail!("a collection has an empty id");
            }
            if collections.iter().any(|k: &Collection| k.id == id) {
                bail!("duplicate collection id {id:?} — ids address a collection and must be unique");
            }

            let shape = match c.node.as_str() {
                "note" => NodeShape::Note,
                "record" => NodeShape::Record,
                other => bail!("{id}: unknown node shape {other:?} — expected note or record"),
            };

            // The title default follows the SHAPE, because a note's title lives
            // in its prose and a record's lives in its facts. A single default
            // would be wrong for one of them on every corpus.
            let title = match c.title {
                Some(raw) => Source::parse(&raw, &format!("{id}.title"))?,
                None => match shape {
                    NodeShape::Note => Source::Heading,
                    NodeShape::Record => Source::Slug,
                },
            };

            let date = match c.date {
                Some(raw) => Some(Source::parse(&raw, &format!("{id}.date"))?),
                None => None,
            };
            let status = match c.status {
                Some(raw) => Some(Source::parse(&raw, &format!("{id}.status"))?),
                None => None,
            };

            let order = match c.order.as_deref() {
                None | Some("title") => Order::Title,
                Some("date-desc") => Order::DateDesc,
                Some("date-asc") => Order::DateAsc,
                Some("slug") => Order::Slug,
                Some(other) => bail!(
                    "{id}: unknown order {other:?} — expected date-desc, date-asc, title or slug"
                ),
            };

            // An order by date with no date source can never sort. Refusing at
            // load is the difference between a named error and a list that is
            // quietly in declaration order while claiming to be chronological.
            if matches!(order, Order::DateDesc | Order::DateAsc) && date.is_none() {
                bail!("{id}: order is by date but no date source is declared");
            }

            let entry = c.entry.unwrap_or_else(|| "index.toml".to_string());
            if entry.contains('/') || entry.contains("..") {
                bail!("{id}: entry {entry:?} must be a bare file name inside the node's directory");
            }

            collections.push(Collection {
                entry,
                label: c.label.unwrap_or_else(|| id.clone()),
                path: root.join(c.path.unwrap_or_else(|| id.clone())),
                id,
                shape,
                title,
                date,
                status,
                order,
                publish: c.publish.unwrap_or(true),
            });
        }

        Ok(Manifest {
            name: raw.corpus.name,
            root,
            recent: raw.overview.recent.unwrap_or(DEFAULT_RECENT),
            collections,
        })
    }

    pub fn collection(&self, id: &str) -> Option<&Collection> {
        self.collections.iter().find(|c| c.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Manifest> {
        Manifest::parse(text, Path::new("/corpus"))
    }

    #[test]
    fn a_note_collection_takes_its_title_from_the_heading_by_default() {
        let m = parse(
            r#"
            [corpus]
            name = "Example"
            [[collection]]
            id = "journal"
            node = "note"
        "#,
        )
        .unwrap();
        let c = m.collection("journal").unwrap();
        assert_eq!(c.title, Source::Heading);
        assert_eq!(c.label, "journal");
        assert_eq!(c.path, Path::new("/corpus/journal"));
        assert!(c.publish);
    }

    #[test]
    fn a_record_collection_defaults_to_the_slug_not_the_heading() {
        // The shape picks the default. One shared default would be wrong for
        // whichever shape did not get it.
        let m = parse(
            r#"
            [corpus]
            name = "Example"
            [[collection]]
            id = "items"
            node = "record"
        "#,
        )
        .unwrap();
        assert_eq!(m.collection("items").unwrap().title, Source::Slug);
    }

    #[test]
    fn an_order_by_date_without_a_date_source_is_refused_at_load() {
        let err = parse(
            r#"
            [corpus]
            name = "Example"
            [[collection]]
            id = "journal"
            node = "note"
            order = "date-desc"
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("no date source"), "{err}");
    }

    #[test]
    fn an_unknown_source_names_the_field_it_came_from() {
        let err = parse(
            r#"
            [corpus]
            name = "Example"
            [[collection]]
            id = "journal"
            node = "note"
            title = "vibes"
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("journal.title"), "{err}");
    }

    #[test]
    fn duplicate_collection_ids_are_refused() {
        let err = parse(
            r#"
            [corpus]
            name = "Example"
            [[collection]]
            id = "a"
            node = "note"
            [[collection]]
            id = "a"
            node = "note"
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("duplicate"), "{err}");
    }

    #[test]
    fn publish_false_survives_the_load() {
        let m = parse(
            r#"
            [corpus]
            name = "Example"
            [[collection]]
            id = "private"
            node = "record"
            publish = false
        "#,
        )
        .unwrap();
        assert!(!m.collection("private").unwrap().publish);
    }

    #[test]
    fn a_corpus_with_no_collections_is_refused() {
        let err = parse(
            r#"
            [corpus]
            name = "Empty"
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("no [[collection]]"), "{err}");
    }
}
