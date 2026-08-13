//! Reading a corpus into nodes, and the links between them.
//!
//! Two hot paths are what this exists to serve: capture a thought, and find a
//! thing again. Only the second one lives here — an index that answers "what is
//! in this corpus and what points at what".
//!
//! ⛔ DETERMINISM. Directory order is whatever the filesystem returns, so every
//! listing here is sorted before it is used. A corpus that renders in a
//! different order on two machines is a bug, not a cosmetic difference: the
//! reader learns the shelf and then the shelf moves.

use crate::manifest::{Collection, Manifest, NodeShape, Order, Source};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Node {
    pub collection: String,
    pub slug: String,
    pub title: String,
    pub date: Option<String>,
    pub status: Option<String>,
    /// Structured facts, when the node has a facts file.
    pub facts: Vec<(String, String)>,
    pub body: Option<String>,
    /// Slugs this node links to, in order of first appearance, deduplicated.
    pub links: Vec<String>,
    pub path: PathBuf,
}

impl Node {
    /// `<collection>/<slug>` — the address a route uses. Unique by
    /// construction: slugs are file stems and a directory holds one stem.
    pub fn address(&self) -> String {
        format!("{}/{}", self.collection, self.slug)
    }
}

pub struct Corpus {
    pub manifest: Manifest,
    pub nodes: Vec<Node>,
}

impl Corpus {
    pub fn load(manifest: Manifest) -> Result<Self> {
        let mut nodes = Vec::new();
        for collection in &manifest.collections {
            nodes.extend(read_collection(collection)?);
        }
        Ok(Corpus { manifest, nodes })
    }

    pub fn in_collection<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a Node> {
        self.nodes.iter().filter(move |n| n.collection == id)
    }

    pub fn count(&self, id: &str) -> usize {
        self.in_collection(id).count()
    }

    pub fn node(&self, address: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.address() == address)
    }

    /// The nodes whose bodies link to `slug`.
    ///
    /// Resolution is corpus-wide and by slug: the owner's settled call is that
    /// a folder boundary is enough and a reference resolves regardless of which
    /// sub-collection its target lives under. A link that resolved only within
    /// a collection would reproduce the containment this design exists to drop.
    pub fn backlinks<'a>(&'a self, slug: &'a str) -> Vec<&'a Node> {
        let mut hits: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|n| n.links.iter().any(|l| l == slug))
            .collect();
        hits.sort_by(|a, b| a.address().cmp(&b.address()));
        hits
    }

    /// A link target that no node answers to. Surfaced rather than hidden: in a
    /// tag-is-a-note system an unresolved link is not an error, it is a note
    /// that has been called for and not yet written.
    pub fn unresolved(&self) -> Vec<String> {
        let known: std::collections::BTreeSet<&str> =
            self.nodes.iter().map(|n| n.slug.as_str()).collect();
        let mut missing: Vec<String> = self
            .nodes
            .iter()
            .flat_map(|n| n.links.iter())
            .filter(|l| !known.contains(l.as_str()))
            .cloned()
            .collect();
        missing.sort();
        missing.dedup();
        missing
    }

    /// A cheap stamp over what is on disk: how many nodes there are and the
    /// newest modification among them.
    ///
    /// ⚠ It is a CHANGE DETECTOR, not a hash of the content. Two different
    /// corpora can collide and that is fine — it is only ever compared against
    /// its own previous value. What it must never do is fail to move when a
    /// node is edited, which is why the newest mtime is in it and not just the
    /// count: an edit that replaces a node leaves the count alone.
    pub fn fingerprint(&self) -> String {
        let newest = self
            .nodes
            .iter()
            .filter_map(|n| std::fs::metadata(&n.path).ok()?.modified().ok())
            .filter_map(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .max()
            .unwrap_or(0);
        format!("{}:{newest}", self.nodes.len())
    }

    /// Most recently dated nodes across every collection that has a date.
    pub fn recent(&self, limit: usize) -> Vec<&Node> {
        let mut dated: Vec<&Node> = self.nodes.iter().filter(|n| n.date.is_some()).collect();
        // Address breaks ties so the order is total. A sort whose comparator
        // can return Equal for two different rows is a listing that may differ
        // between runs on the same input.
        dated.sort_by(|a, b| {
            b.date
                .cmp(&a.date)
                .then_with(|| a.address().cmp(&b.address()))
        });
        dated.truncate(limit);
        dated
    }
}

fn read_collection(collection: &Collection) -> Result<Vec<Node>> {
    if !collection.path.is_dir() {
        // A declared collection whose directory is absent is empty, not fatal:
        // a manifest describes the shape a corpus may take, and a corpus is
        // allowed to not have started a collection yet.
        return Ok(Vec::new());
    }

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&collection.path)
        .with_context(|| format!("reading {}", collection.path.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();

    let mut nodes = Vec::new();
    for entry in entries {
        let Some(node) = read_node(collection, &entry)? else {
            continue;
        };
        nodes.push(node);
    }

    sort_nodes(&mut nodes, collection.order);
    Ok(nodes)
}

fn read_node(collection: &Collection, entry: &Path) -> Result<Option<Node>> {
    let (slug, facts_path, body_path) = match collection.shape {
        NodeShape::Note => {
            if entry.extension().and_then(|e| e.to_str()) != Some("md") {
                return Ok(None);
            }
            let Some(slug) = stem(entry) else {
                return Ok(None);
            };
            (slug, None, Some(entry.to_path_buf()))
        }
        NodeShape::Record if entry.is_dir() => {
            let facts = entry.join(&collection.entry);
            if !facts.is_file() {
                return Ok(None);
            }
            let Some(slug) = stem(entry) else {
                return Ok(None);
            };
            // The prose file sits beside the facts file and shares its stem —
            // `matter.toml` is accompanied by `matter.md`, not by `index.md`.
            let body = facts.with_extension("md");
            (slug, Some(facts), body.is_file().then_some(body))
        }
        NodeShape::Record => {
            if entry.extension().and_then(|e| e.to_str()) != Some("toml") {
                return Ok(None);
            }
            let Some(slug) = stem(entry) else {
                return Ok(None);
            };
            let body = entry.with_extension("md");
            (
                slug,
                Some(entry.to_path_buf()),
                body.is_file().then_some(body),
            )
        }
    };

    let body = match &body_path {
        Some(p) => Some(std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?),
        None => None,
    };
    let facts = match &facts_path {
        Some(p) => {
            let text =
                std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
            Some(toml::from_str::<toml::Value>(&text).with_context(|| format!("parsing {}", p.display()))?)
        }
        None => None,
    };

    let prose = body.as_deref().map(strip_frontmatter).unwrap_or("");
    let frontmatter = body.as_deref().map(read_frontmatter).unwrap_or_default();

    let ctx = Fields {
        slug: &slug,
        prose,
        frontmatter: &frontmatter,
        facts: facts.as_ref(),
        path: entry,
    };

    Ok(Some(Node {
        collection: collection.id.clone(),
        title: ctx
            .resolve(&collection.title)
            .unwrap_or_else(|| prettify(&slug)),
        date: collection.date.as_ref().and_then(|s| ctx.resolve(s)),
        status: collection.status.as_ref().and_then(|s| ctx.resolve(s)),
        facts: facts.as_ref().map(flatten_facts).unwrap_or_default(),
        links: extract_links(prose),
        body: (!prose.trim().is_empty()).then(|| prose.to_string()),
        slug,
        path: entry.to_path_buf(),
    }))
}

fn stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty() && !s.starts_with('.'))
}

struct Fields<'a> {
    slug: &'a str,
    prose: &'a str,
    frontmatter: &'a BTreeMap<String, String>,
    facts: Option<&'a toml::Value>,
    path: &'a Path,
}

impl Fields<'_> {
    fn resolve(&self, source: &Source) -> Option<String> {
        let value = match source {
            Source::Heading => first_heading(self.prose)?,
            Source::Frontmatter(key) => self.frontmatter.get(key)?.clone(),
            Source::Facts(key) => fact(self.facts?, key)?,
            Source::Filename => leading_date(self.slug)?,
            Source::Slug => prettify(self.slug),
            Source::Mtime => {
                let modified = std::fs::metadata(self.path).ok()?.modified().ok()?;
                let secs = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_secs();
                secs.to_string()
            }
        };
        (!value.trim().is_empty()).then(|| value.trim().to_string())
    }
}

fn first_heading(prose: &str) -> Option<String> {
    prose
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|h| h.trim().to_string())
}

/// A leading `YYYY-MM-DD`, which is how a dated entry names itself. Anything
/// else returns None rather than a guess — a date the engine invented would
/// sort a listing and never be questioned.
fn leading_date(slug: &str) -> Option<String> {
    let bytes = slug.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let shape = |i: usize| bytes[i].is_ascii_digit();
    let ok = (0..4).all(shape)
        && bytes[4] == b'-'
        && (5..7).all(shape)
        && bytes[7] == b'-'
        && (8..10).all(shape);
    ok.then(|| slug[..10].to_string())
}

fn prettify(slug: &str) -> String {
    slug.replace(['-', '_'], " ")
}

/// A dotted key walks nested tables, so a corpus that keeps a name under a
/// sub-table does not need the engine to learn its layout.
fn fact(facts: &toml::Value, key: &str) -> Option<String> {
    let mut cursor = facts;
    for part in key.split('.') {
        cursor = cursor.get(part)?;
    }
    scalar(cursor)
}

fn scalar(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        toml::Value::Datetime(d) => Some(d.to_string()),
        toml::Value::Array(items) => {
            let parts: Vec<String> = items.iter().filter_map(scalar).collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        toml::Value::Table(_) => None,
    }
}

/// Facts as display pairs, nested tables dotted. Sorted, because a TOML table's
/// iteration order is not the file's order and a panel that reshuffles between
/// runs is unreadable.
fn flatten_facts(facts: &toml::Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk_facts(facts, "", &mut out);
    out.sort();
    out
}

fn walk_facts(value: &toml::Value, prefix: &str, out: &mut Vec<(String, String)>) {
    let toml::Value::Table(table) = value else {
        return;
    };
    for (key, item) in table {
        let name = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match item {
            toml::Value::Table(_) => walk_facts(item, &name, out),
            _ => {
                if let Some(text) = scalar(item) {
                    out.push((name, text));
                }
            }
        }
    }
}

fn read_frontmatter(body: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(rest) = body.strip_prefix("---\n") else {
        return map;
    };
    let Some(end) = rest.find("\n---") else {
        return map;
    };
    for line in rest[..end].lines() {
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !key.trim().is_empty() {
                map.insert(key.trim().to_string(), value.to_string());
            }
        }
    }
    map
}

fn strip_frontmatter(body: &str) -> &str {
    let Some(rest) = body.strip_prefix("---\n") else {
        return body;
    };
    match rest.find("\n---") {
        // 4 = the newline plus the three dashes; then skip to the line after.
        Some(end) => rest[end + 4..].trim_start_matches('\n'),
        None => body,
    }
}

/// `[[target]]` and `[[target|label]]`, in order of first appearance.
///
/// The link is the whole retrieval story in a tag-is-a-note system, so it is
/// extracted from the prose rather than from a metadata field: a writer types
/// the link mid-sentence and must never have to also declare it somewhere else.
fn extract_links(prose: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = prose;
    while let Some(open) = rest.find("[[") {
        rest = &rest[open + 2..];
        let Some(close) = rest.find("]]") else { break };
        let target = rest[..close].split('|').next().unwrap_or("").trim();
        if !target.is_empty() && !links.iter().any(|l| l == target) {
            links.push(target.to_string());
        }
        rest = &rest[close + 2..];
    }
    links
}

fn sort_nodes(nodes: &mut [Node], order: Order) {
    match order {
        // Every comparator ends in the slug so the order is total. Two nodes
        // that compare equal on the declared key would otherwise land in
        // readdir order, which is the non-determinism this file exists to
        // avoid, hidden one level deeper.
        Order::Title => nodes.sort_by(|a, b| {
            a.title
                .to_lowercase()
                .cmp(&b.title.to_lowercase())
                .then_with(|| a.slug.cmp(&b.slug))
        }),
        Order::Slug => nodes.sort_by(|a, b| a.slug.cmp(&b.slug)),
        Order::DateAsc => nodes.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.slug.cmp(&b.slug))),
        Order::DateDesc => {
            nodes.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.slug.cmp(&b.slug)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Corpus {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name);
        let manifest = Manifest::load(&root.join("kasten.toml")).unwrap();
        Corpus::load(manifest).unwrap()
    }

    #[test]
    fn a_note_corpus_reads_titles_from_headings_and_dates_from_filenames() {
        let c = fixture("fieldbook");
        let journal: Vec<&Node> = c.in_collection("journal").collect();
        assert_eq!(journal.len(), 3);
        // date-desc: the newest entry leads.
        assert_eq!(journal[0].title, "Nightjar at dusk");
        assert_eq!(journal[0].date.as_deref(), Some("2031-04-02"));
        assert_eq!(journal[2].date.as_deref(), Some("2031-03-04"));
    }

    #[test]
    fn a_record_corpus_reads_titles_from_facts_and_finds_directory_nodes() {
        let c = fixture("atlas");
        let instruments: Vec<&Node> = c.in_collection("instruments").collect();
        assert_eq!(instruments.len(), 2);
        // One is a flat pair, one is a directory holding index.toml — both are
        // nodes and neither needed the engine to know which corpus this is.
        let gauge = c.node("instruments/tide-gauge-04").unwrap();
        assert_eq!(gauge.title, "Tide Gauge 04");
        assert_eq!(gauge.status.as_deref(), Some("durable"));
        assert!(gauge.body.is_some());
        assert!(gauge.facts.iter().any(|(k, v)| k == "serial" && v == "TG-0004"));
    }

    #[test]
    fn links_resolve_across_collections_not_only_within_one() {
        // The settled call: a folder boundary is enough, and a reference
        // resolves regardless of which sub-collection its target is under.
        let c = fixture("fieldbook");
        let backlinks = c.backlinks("amphibians");
        assert_eq!(backlinks.len(), 2);
        assert!(backlinks.iter().all(|n| n.collection == "journal"));

        let across = c.backlinks("field-methods");
        let collections: Vec<&str> = across.iter().map(|n| n.collection.as_str()).collect();
        assert!(
            collections.contains(&"journal") && collections.contains(&"characters"),
            "expected backlinks from two different collections, got {collections:?}"
        );
    }

    #[test]
    fn a_link_to_a_note_that_does_not_exist_yet_is_reported_not_swallowed() {
        let c = fixture("atlas");
        // atlas links harbour-light <-> tide-gauge-04, both of which exist.
        assert!(c.unresolved().is_empty(), "{:?}", c.unresolved());

        let c = fixture("fieldbook");
        assert!(c.unresolved().is_empty(), "{:?}", c.unresolved());
    }

    #[test]
    fn the_unresolved_report_can_actually_report() {
        // Negative control for the test above: prove the instrument can say
        // DIRTY, or its CLEAN means nothing.
        let links = extract_links("a link to [[nowhere]] and one to [[also-nowhere|labelled]]");
        assert_eq!(links, vec!["nowhere", "also-nowhere"]);
    }

    #[test]
    fn recent_spans_collections_and_ignores_undated_ones() {
        let c = fixture("fieldbook");
        let recent = c.recent(10);
        // Only the journal declares a date source, so characters and indices
        // cannot appear here however recently they were touched.
        assert!(recent.iter().all(|n| n.collection == "journal"));
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].slug, "2031-04-02-nightjar-at-dusk");
    }

    #[test]
    fn frontmatter_is_read_and_then_kept_out_of_the_body() {
        let body = "---\ntitle: Declared\nstatus: durable\n---\n\n# Heading\n\nProse.\n";
        let fm = read_frontmatter(body);
        assert_eq!(fm.get("title").map(String::as_str), Some("Declared"));
        let prose = strip_frontmatter(body);
        assert!(prose.starts_with("# Heading"), "{prose:?}");
        assert!(!prose.contains("status:"));
    }

    #[test]
    fn a_filename_without_a_date_yields_none_rather_than_a_guess() {
        assert_eq!(leading_date("2031-03-04-first-thaw").as_deref(), Some("2031-03-04"));
        assert_eq!(leading_date("first-thaw"), None);
        assert_eq!(leading_date("20310304-first-thaw"), None);
    }

    #[test]
    fn ordering_is_total_so_two_runs_agree() {
        let c = fixture("atlas");
        let first: Vec<String> = c.nodes.iter().map(Node::address).collect();
        let c2 = fixture("atlas");
        let second: Vec<String> = c2.nodes.iter().map(Node::address).collect();
        assert_eq!(first, second);
    }
}
