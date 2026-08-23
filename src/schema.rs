//! The widget schema — this app's entire user interface.
//!
//! ⛔ Tier A: the host paints everything here and this crate ships no UI code.
//! Collections, rows, prose, counts and a search box are all vocabulary the
//! host already has, so nothing in this file draws a pixel; it describes.
//!
//! The forbidden move, written down because it is the one that will be tempting
//! the first time a corpus wants a picture of its links: do NOT reach for a
//! native surface because a widget is missing. That serves one app and charges
//! every app the foreign-engine tax forever. A missing widget is a vocabulary
//! gap to file with the host, which then has it for everyone.

use crate::corpus::{Corpus, Node};
use serde_json::{json, Value};

/// What the reading surface is showing. Parsed from a string so a route is
/// addressable from the CLI and provable without a GUI in reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Home,
    Vault(String),
    Collection(String),
    Node(String),
}

impl Route {
    pub fn parse(raw: &str) -> Route {
        match raw.split_once(':') {
            Some(("collection", id)) => Route::Collection(id.to_string()),
            Some(("vault", id)) => Route::Vault(id.to_string()),
            Some(("node", address)) => Route::Node(address.to_string()),
            _ => Route::Home,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Route::Home => "home".to_string(),
            Route::Vault(id) => format!("vault:{id}"),
            Route::Collection(id) => format!("collection:{id}"),
            Route::Node(address) => format!("node:{address}"),
        }
    }
}

/// The viewport pane: whatever is being read right now.
pub fn document(corpus: &Corpus, route: &Route, search: &str) -> Value {
    let widgets = match route {
        Route::Home => home(corpus),
        Route::Vault(id) => vault(corpus, id),
        Route::Collection(id) => collection(corpus, id, search),
        Route::Node(address) => node(corpus, address),
    };
    json!({
        "title": corpus.manifest.name,
        "widgets": widgets,
        "footer": footer(corpus, route),
    })
}

fn vault(corpus: &Corpus, id: &str) -> Vec<Value> {
    let selected: Vec<_> = corpus
        .manifest
        .collections
        .iter()
        .filter(|c| c.vault_id.as_deref() == Some(id))
        .collect();
    let Some(first) = selected.first() else {
        return vec![
            json!({ "kind": "label", "muted": true, "text": format!("No vault {id:?}.") }),
        ];
    };
    let mut widgets =
        vec![json!({ "kind": "section", "text": first.vault.as_deref().unwrap_or(id) })];
    for c in selected {
        widgets.push(json!({
            "kind": "list-row", "id": format!("collection-{}", c.id),
            "title": c.label, "subtitle": format!("{} nodes", corpus.count(&c.id)),
            "actions": [{ "action": format!("open:collection:{}", c.id), "label": "⤢", "title": "Open this collection" }],
        }));
    }
    widgets
}

fn home(corpus: &Corpus) -> Vec<Value> {
    let mut widgets = Vec::new();
    let mut current_vault: Option<&str> = None;

    for c in &corpus.manifest.collections {
        let vault = c.vault.as_deref();
        if vault != current_vault {
            widgets.push(json!({ "kind": "section", "text": vault.unwrap_or("Collections") }));
            current_vault = vault;
        }
        let count = corpus.count(&c.id);
        let mut subtitle = format!("{count} {}", if count == 1 { "node" } else { "nodes" });
        // A collection that may never be published says so on its own row. The
        // property is declared in the manifest and carried to the surface,
        // rather than being a thing the reader is expected to remember about a
        // directory.
        if !c.publish {
            subtitle.push_str(" · never published");
        }
        widgets.push(json!({
            "kind": "list-row",
            "id": format!("collection-{}", c.id),
            "title": c.label,
            "subtitle": subtitle,
            "actions": [{ "action": format!("open:collection:{}", c.id), "label": "⤢", "title": "Open this collection" }],
        }));
    }

    let recent = corpus.recent(corpus.manifest.recent);
    if !recent.is_empty() {
        widgets.push(json!({ "kind": "section", "text": "Recent" }));
        widgets.extend(recent.into_iter().map(|n| node_row(corpus, n, true)));
    }

    // A link with no note behind it is not an error in a tag-is-a-note system;
    // it is a note that has been called for and not written. Showing it here is
    // the cheapest retrieval affordance the corpus has: it is a to-write list
    // the writer produced by writing, at a cost of zero extra keystrokes.
    let wanted = corpus.unresolved();
    if !wanted.is_empty() {
        widgets.push(json!({ "kind": "section", "text": "Wanted" }));
        for slug in wanted {
            widgets.push(json!({
                "kind": "list-row",
                "id": format!("wanted-{slug}"),
                "title": slug,
                "subtitle": "linked to, not yet written",
            }));
        }
    }

    widgets
}

fn collection(corpus: &Corpus, id: &str, search: &str) -> Vec<Value> {
    let Some(declared) = corpus.manifest.collection(id) else {
        return vec![json!({
            "kind": "label", "muted": true,
            "text": format!("No collection {id:?} in this corpus."),
        })];
    };

    let mut widgets = vec![
        json!({
            "kind": "search-box", "id": "search", "action": "search",
            "placeholder": format!("Search {}…", declared.label),
            "value": search,
        }),
        json!({ "kind": "section", "text": declared.label }),
    ];

    let needle = search.trim().to_lowercase();
    let mut shown = 0usize;
    for n in corpus.in_collection(id) {
        if !needle.is_empty() && !matches(n, &needle) {
            continue;
        }
        widgets.push(node_row(corpus, n, false));
        shown += 1;
    }

    if shown == 0 {
        widgets.push(json!({
            "kind": "label", "muted": true,
            "text": if needle.is_empty() { "Nothing here yet.".to_string() }
                    else { format!("Nothing matches {search:?}.") },
        }));
    }

    widgets
}

/// Title, slug and prose are all searched. A search that only reads titles
/// answers "did you name it well", which is not the question a writer has
/// months later — they remember a phrase, not a filename.
fn matches(node: &Node, needle: &str) -> bool {
    node.title.to_lowercase().contains(needle)
        || node.slug.to_lowercase().contains(needle)
        || node
            .body
            .as_deref()
            .is_some_and(|b| b.to_lowercase().contains(needle))
        || node
            .facts
            .iter()
            .any(|(k, v)| k.to_lowercase().contains(needle) || v.to_lowercase().contains(needle))
}

fn node(corpus: &Corpus, address: &str) -> Vec<Value> {
    let Some(n) = corpus.node(address) else {
        return vec![json!({
            "kind": "label", "muted": true,
            "text": format!("No node at {address:?}."),
        })];
    };

    let mut widgets = vec![json!({ "kind": "section", "text": n.title })];

    if !n.facts.is_empty() {
        // `card: true` is opt-in and this is what it is for: a short block of
        // structured fields reads as a form. A card around a long list of rows
        // would be the nested-card stack the design rules out.
        widgets.push(json!({ "kind": "section", "text": "Facts", "card": true }));
        for (key, value) in &n.facts {
            widgets.push(json!({
                "kind": "label",
                "text": format!("{key}: {value}"),
            }));
        }
    }

    if let Some(body) = &n.body {
        // The markdown widget hands the source to the host, which renders it
        // through the platform's renderer. This app must never grow markdown
        // handling of its own — that would break the property that makes the
        // pipeline worth having: improve the organ once, every consumer
        // improves.
        widgets.push(json!({ "kind": "markdown", "id": "body", "source": body }));
    }

    let backlinks = corpus.backlinks(&n.slug);
    if !backlinks.is_empty() {
        widgets.push(json!({ "kind": "section", "text": "Linked from" }));
        widgets.extend(backlinks.into_iter().map(|b| node_row(corpus, b, true)));
    }

    let outbound: Vec<&Node> = n
        .links
        .iter()
        .flat_map(|slug| corpus.nodes.iter().filter(move |c| &c.slug == slug))
        .collect();
    if !outbound.is_empty() {
        widgets.push(json!({ "kind": "section", "text": "Links to" }));
        widgets.extend(outbound.into_iter().map(|t| node_row(corpus, t, true)));
    }

    widgets
}

/// One node as a row. `qualify` prefixes the collection label, for lists that
/// span collections — the same row in its own collection's list does not need
/// to repeat where it is.
fn node_row(corpus: &Corpus, n: &Node, qualify: bool) -> Value {
    let mut parts: Vec<String> = Vec::new();
    if qualify {
        let label = corpus
            .manifest
            .collection(&n.collection)
            .map(|c| match &c.vault {
                Some(vault) => format!("{vault} / {}", c.label),
                None => c.label.clone(),
            })
            .unwrap_or_else(|| n.collection.clone());
        parts.push(label);
    }
    if let Some(date) = &n.date {
        parts.push(date.clone());
    }

    let mut row = json!({
        "kind": "list-row",
        "id": format!("node-{}", n.address()),
        "title": n.title,
        "subtitle": parts.join(" · "),
        "actions": [{ "action": format!("open:node:{}", n.address()), "label": "⤢", "title": "Open this node" }],
    });

    // The app names a durability CLASS and the host owns the colour. A token
    // the host does not paint leaves the slot empty rather than failing the
    // pane, so a corpus is free to use its own status vocabulary and get no dot
    // for the values that are not about durability.
    if let Some(status) = &n.status {
        if status == "durable" || status == "transient" {
            row["status"] = json!(status);
        }
    }

    row
}

fn footer(corpus: &Corpus, route: &Route) -> Vec<Value> {
    let nodes = corpus.nodes.len();
    let collections = corpus.manifest.collections.len();
    let mut footer = vec![json!({
        "kind": "label",
        "text": format!(
            "{} · {nodes} node{} in {collections} collection{}",
            corpus.manifest.name,
            if nodes == 1 { "" } else { "s" },
            if collections == 1 { "" } else { "s" },
        ),
    })];
    if !matches!(route, Route::Home) {
        footer.push(json!({
            "kind": "button", "id": "home", "label": "Overview", "action": "open:home",
        }));
    }
    footer
}

/// The rail pane: navigation that stays put while the viewport changes.
pub fn navigation(
    corpus: &Corpus,
    route: &Route,
    capture_error: Option<&(String, String)>,
) -> Value {
    let mut widgets = Vec::new();

    // Capture comes FIRST, above navigation, because it is the first hot path
    // and the one the design value ranks. A thought arriving while the reader
    // is three clicks deep in a collection must not require them to go
    // anywhere — the box is in the same place on every route.
    //
    // Absent from a corpus that declares no capture target: an app that offered
    // to take someone's writing and then had nowhere to put it would be worse
    // than one that never offered.
    if corpus.manifest.capture.is_some() {
        // On a failed capture the box is re-declared HOLDING the writer's own
        // words, with the reason above it. An empty box after a failed write is
        // indistinguishable from a successful one, and the difference is a
        // thought they will not get back.
        let (value, error) = match capture_error {
            Some((text, message)) => (text.as_str(), Some(message.as_str())),
            None => ("", None),
        };
        if let Some(message) = error {
            widgets.push(json!({
                "kind": "label", "muted": true,
                "text": format!("⚠ not captured — {message}"),
            }));
        }
        widgets.push(json!({
            "kind": "text-input", "id": "capture", "action": "capture",
            "placeholder": "Capture a thought…", "value": value,
        }));
    }

    widgets.push(json!({
        "kind": "button", "id": "overview", "label": "Overview",
        "action": "open:home", "primary": matches!(route, Route::Home),
    }));
    let mut vaults = Vec::new();
    for c in &corpus.manifest.collections {
        if let (Some(id), Some(label)) = (c.vault_id.as_deref(), c.vault.as_deref()) {
            if !vaults.iter().any(|(seen, _)| *seen == id) {
                vaults.push((id, label));
            }
        }
    }
    if !vaults.is_empty() {
        widgets.push(json!({ "kind": "section", "text": "Vaults" }));
        for (id, label) in vaults {
            let selected = matches!(route, Route::Vault(current) if current == id);
            let count: usize = corpus
                .manifest
                .collections
                .iter()
                .filter(|c| c.vault_id.as_deref() == Some(id))
                .map(|c| corpus.count(&c.id))
                .sum();
            widgets.push(json!({
                "kind": "list-row", "id": format!("vault-{id}"), "title": label,
                "subtitle": format!("{count} notes"), "selected": selected,
                "actions": [{ "action": format!("open:vault:{id}"), "label": "⤢", "title": "Open this vault" }],
            }));
        }
    }
    let active_vault = match route {
        Route::Vault(id) => Some(id.as_str()),
        Route::Collection(id) => corpus
            .manifest
            .collection(id)
            .and_then(|c| c.vault_id.as_deref()),
        Route::Node(address) => address
            .split_once('/')
            .and_then(|(id, _)| corpus.manifest.collection(id))
            .and_then(|c| c.vault_id.as_deref()),
        Route::Home => None,
    };
    let mut current_vault: Option<&str> = None;

    for c in corpus.manifest.collections.iter().filter(|c| {
        c.vault_id.is_none() || active_vault.is_some_and(|id| c.vault_id.as_deref() == Some(id))
    }) {
        let vault = c.vault.as_deref();
        if vault != current_vault {
            widgets.push(json!({ "kind": "section", "text": vault.unwrap_or("Collections") }));
            current_vault = vault;
        }
        let selected = matches!(route, Route::Collection(id) if id == &c.id);
        widgets.push(json!({
            "kind": "list-row",
            "id": format!("nav-{}", c.id),
            "title": c.label,
            "subtitle": corpus.count(&c.id).to_string(),
            "selected": selected,
            "actions": [{ "action": format!("open:collection:{}", c.id), "label": "⤢", "title": "Open this collection" }],
        }));
    }

    json!({ "title": corpus.manifest.name, "widgets": widgets })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Corpus {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name);
        let manifest = Manifest::load(&root.join("kasten.toml")).unwrap();
        Corpus::load(manifest).unwrap()
    }

    fn kinds(schema: &Value) -> Vec<String> {
        schema["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w["kind"].as_str().unwrap_or("?").to_string())
            .collect()
    }

    /// THE FALSIFIER FOR THE WHOLE DESIGN: one binary, two corpora with no
    /// vocabulary in common, rendered from their manifests alone.
    #[test]
    fn the_same_code_renders_two_corpora_that_share_no_vocabulary() {
        let a = fixture("fieldbook");
        let b = fixture("atlas");

        let a_ids: Vec<&str> = a
            .manifest
            .collections
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        let b_ids: Vec<&str> = b
            .manifest
            .collections
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert!(
            a_ids.iter().all(|id| !b_ids.contains(id)),
            "the fixtures must share no collection id, or this proves nothing: {a_ids:?} vs {b_ids:?}"
        );

        for corpus in [&a, &b] {
            let home = document(corpus, &Route::Home, "");
            assert_eq!(home["title"], corpus.manifest.name);
            let rows = home["widgets"].as_array().unwrap().len();
            assert!(
                rows > corpus.manifest.collections.len(),
                "home rendered nothing"
            );
        }
    }

    #[test]
    fn a_never_published_collection_says_so_on_its_row() {
        let atlas = fixture("atlas");
        let home = document(&atlas, &Route::Home, "");
        let row = home["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["id"] == "collection-expeditions")
            .expect("expeditions row");
        assert!(
            row["subtitle"]
                .as_str()
                .unwrap()
                .contains("never published"),
            "{row}"
        );
    }

    #[test]
    fn a_node_view_carries_facts_prose_and_both_link_directions() {
        let atlas = fixture("atlas");
        let schema = document(&atlas, &Route::Node("venues/harbour-light".into()), "");
        let kinds = kinds(&schema);
        assert!(kinds.contains(&"markdown".to_string()), "{kinds:?}");
        assert!(kinds.contains(&"section".to_string()));

        let text = schema.to_string();
        assert!(text.contains("Facts"), "facts card missing");
        assert!(text.contains("Linked from"), "backlinks missing");
        assert!(text.contains("Links to"), "outbound links missing");
    }

    #[test]
    fn search_reads_the_prose_not_only_the_title() {
        let field = fixture("fieldbook");
        // "culvert" is in two bodies and one title.
        let hits = document(&field, &Route::Collection("journal".into()), "culvert");
        let rows = hits["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|w| w["kind"] == "list-row")
            .count();
        assert_eq!(rows, 2, "{hits}");
    }

    #[test]
    fn a_search_that_matches_nothing_says_so_rather_than_rendering_an_empty_list() {
        // Negative control for the test above: prove the search can miss.
        let field = fixture("fieldbook");
        let schema = document(
            &field,
            &Route::Collection("journal".into()),
            "zzzz-no-such-word",
        );
        let rows = schema["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|w| w["kind"] == "list-row")
            .count();
        assert_eq!(rows, 0);
        assert!(schema.to_string().contains("Nothing matches"), "{schema}");
    }

    #[test]
    fn only_the_two_durability_tokens_reach_the_status_slot() {
        let atlas = fixture("atlas");
        let schema = document(&atlas, &Route::Collection("instruments".into()), "");
        let row = schema["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["id"] == "node-instruments/tide-gauge-04")
            .unwrap();
        assert_eq!(row["status"], "durable");

        // A corpus whose status vocabulary is its own gets an empty slot, never
        // a guessed colour and never a failed pane.
        let field = fixture("fieldbook");
        let schema = document(&field, &Route::Collection("journal".into()), "");
        for row in schema["widgets"].as_array().unwrap() {
            assert!(row.get("status").is_none(), "{row}");
        }
    }

    #[test]
    fn an_unknown_route_target_reports_instead_of_failing_the_pane() {
        let field = fixture("fieldbook");
        let schema = document(&field, &Route::Node("journal/no-such-note".into()), "");
        assert!(schema.to_string().contains("No node at"), "{schema}");
        let schema = document(&field, &Route::Collection("no-such-collection".into()), "");
        assert!(schema.to_string().contains("No collection"), "{schema}");
    }

    #[test]
    fn a_route_round_trips_through_its_string_form() {
        for route in [
            Route::Home,
            Route::Vault("example".into()),
            Route::Collection("journal".into()),
            Route::Node("journal/2031-03-04-first-thaw".into()),
        ] {
            assert_eq!(Route::parse(&route.as_string()), route);
        }
    }

    /// The loss case, asserted: a failed write must give the writer their words
    /// back. An empty box after a failure is indistinguishable from a success,
    /// and the difference is a thought they cannot recover.
    #[test]
    fn a_failed_capture_puts_the_writers_words_back_in_the_box() {
        let field = fixture("fieldbook");
        let failed = (
            "the thought that did not make it".to_string(),
            "writing /nope: read-only file system".to_string(),
        );
        let nav = navigation(&field, &Route::Home, Some(&failed));
        let rows = nav["widgets"].as_array().unwrap();

        let box_widget = rows
            .iter()
            .find(|w| w["id"] == "capture")
            .expect("capture box");
        assert_eq!(box_widget["value"], "the thought that did not make it");
        assert!(
            rows.iter().any(|w| w["kind"] == "label"
                && w["text"].as_str().unwrap_or("").contains("not captured")
                && w["text"].as_str().unwrap_or("").contains("read-only")),
            "the reason must be on screen beside the words: {nav}"
        );

        // NEGATIVE CONTROL: with no failure the box is empty and carries no
        // warning, or the assertions above would pass on every render.
        let clean = navigation(&field, &Route::Home, None);
        let rows = clean["widgets"].as_array().unwrap();
        assert_eq!(
            rows.iter().find(|w| w["id"] == "capture").unwrap()["value"],
            ""
        );
        assert!(!clean.to_string().contains("not captured"));
    }

    #[test]
    fn the_rail_marks_the_collection_being_read() {
        let field = fixture("fieldbook");
        let nav = navigation(&field, &Route::Collection("indices".into()), None);
        let rows = nav["widgets"].as_array().unwrap();
        let selected: Vec<&Value> = rows.iter().filter(|w| w["selected"] == true).collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0]["id"], "nav-indices");
    }
}
