//! The control endpoint: `GET /pane/<id>` for a schema, `POST /action` for
//! everything the reader does in it. Hand-rolled HTTP over a `TcpListener` —
//! one tiny request shape, no framework, loopback only.
//!
//! ⛔ Loopback only, and deliberately. The corpora this reads are private; a
//! surface bound to any other interface would publish one over the network to
//! whoever asked. The host reaches a remote app through the session's own ssh
//! forwarding, which is the same thing without the open port.
//!
//! ⛔ NO SECRETS IN A SCHEMA, EVER. A schema is fetched by the host and painted;
//! it is not a private channel.

use crate::corpus::Corpus;
use crate::manifest::Manifest;
use crate::schema::{self, Route};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Everything the surface is showing, and nothing about how it looks.
pub struct State {
    pub manifest_path: PathBuf,
    pub corpus: Corpus,
    pub route: Route,
    pub search: String,
}

impl State {
    /// A stamp over what the panes would render. The host refetches a schema
    /// only when this moves, so it must change whenever the content does — and
    /// must NOT change otherwise, or every heartbeat drags a full refetch.
    ///
    /// Route and search are what the reader changed; the corpus fingerprint is
    /// what someone else changed on disk while they were reading.
    pub fn document_version(&self) -> String {
        format!(
            "{}|{}|{}",
            self.route.as_string(),
            self.search,
            self.corpus.fingerprint()
        )
    }

    /// Re-read the corpus from disk.
    ///
    /// ⚠ Done on every action rather than watched, because the writing happens
    /// in an editor and this surface is the reader: an entry captured a second
    /// ago must be here when the reader looks. The cost is a re-scan per
    /// gesture, which is nothing at fixture scale and is the first thing to
    /// measure — not guess at — if a real corpus ever feels slow.
    pub fn reload(&mut self) {
        let Ok(manifest) = Manifest::load(&self.manifest_path) else {
            return;
        };
        if let Ok(corpus) = Corpus::load(manifest) {
            self.corpus = corpus;
        }
    }
}

pub struct Server {
    pub url: String,
    pub state: Arc<Mutex<State>>,
}

pub fn spawn(state: State) -> Result<Server> {
    let listener = TcpListener::bind("127.0.0.1:0").context("binding the kasten control server")?;
    let port = listener.local_addr()?.port();
    let state = Arc::new(Mutex::new(state));
    {
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            for incoming in listener.incoming() {
                let Ok(stream) = incoming else { continue };
                let state = Arc::clone(&state);
                std::thread::spawn(move || handle(stream, &state));
            }
        });
    }
    Ok(Server {
        url: format!("http://127.0.0.1:{port}"),
        state,
    })
}

fn handle(stream: TcpStream, state: &Mutex<State>) {
    let Ok(peek) = stream.try_clone() else { return };
    let mut reader = BufReader::new(peek);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let path = target.split('?').next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let body: Value = if content_length > 0 {
        let mut raw = vec![0u8; content_length];
        if reader.read_exact(&mut raw).is_err() {
            return;
        }
        serde_json::from_slice(&raw).unwrap_or(Value::Null)
    } else {
        Value::Null
    };

    match (method.as_str(), path.as_str()) {
        ("GET", "/pane/doc") => {
            let s = state.lock().unwrap();
            respond(stream, 200, &schema::document(&s.corpus, &s.route, &s.search));
        }
        ("GET", "/pane/nav") => {
            let s = state.lock().unwrap();
            respond(stream, 200, &schema::navigation(&s.corpus, &s.route));
        }
        ("POST", "/action") => {
            let mut s = state.lock().unwrap();
            apply(&mut s, &body);
            let version = s.document_version();
            respond(stream, 200, &json!({ "ok": true, "document_version": version }));
        }
        _ => respond(stream, 404, &json!({ "ok": false })),
    }
}

/// The action grammar is `open:<route>` plus `search`.
///
/// The route rides in the ACTION NAME rather than in a second field, because a
/// widget id is a slot in a pane and not a domain identity — encoding "which
/// node" in the id would make two different things share one encoding, and the
/// host is free to namespace an id whenever it likes.
pub fn apply(state: &mut State, body: &Value) {
    let action = body["action"].as_str().unwrap_or_default();
    let values = &body["values"];

    if let Some(route) = action.strip_prefix("open:") {
        state.route = Route::parse(route);
        // A route change clears the filter. Carrying a search from one
        // collection into the next shows an unexplained near-empty list, and
        // the reader has to work out why — which is a resistance the design
        // value ranks above almost everything else.
        state.search.clear();
        state.reload();
        return;
    }

    if action == "capture" {
        let text = values["capture"]
            .as_str()
            .or_else(|| values["value"].as_str())
            .unwrap_or_default()
            .to_string();
        // An empty box is a stray Enter, not a thought. Refusing quietly is
        // right here: a warning for pressing Enter on an empty field would be
        // the app scolding the writer for nothing.
        if !text.trim().is_empty() {
            let now = chrono::Local::now();
            let today = now.format("%Y-%m-%d").to_string();
            let at = now.format("%H:%M").to_string();
            // A failed capture must never be silent — the writer believes the
            // thought is filed and it is not. There is no user-visible error
            // channel on this surface yet, so it goes to stderr where the
            // session that launched the app will hold it.
            match crate::capture::write(&state.corpus.manifest, &today, &at, &text) {
                Ok(out) => eprintln!("kasten: captured to {}", out.path.display()),
                Err(error) => eprintln!("kasten: CAPTURE FAILED — {error:#}"),
            }
        }
        state.reload();
        return;
    }

    if action == "search" {
        state.search = values["search"]
            .as_str()
            .or_else(|| values["value"].as_str())
            .unwrap_or_default()
            .to_string();
        return;
    }

    // An unknown action leaves the surface exactly as it was. A pane that
    // reset itself on an action it did not recognise would lose the reader's
    // place for a reason they cannot see.
    state.reload();
}

fn respond(mut stream: TcpStream, status: u16, value: &Value) {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    let reason = if status == 200 { "OK" } else { "Not Found" };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(name: &str) -> State {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name);
        let manifest_path = root.join("kasten.toml");
        let corpus = Corpus::load(Manifest::load(&manifest_path).unwrap()).unwrap();
        State {
            manifest_path,
            corpus,
            route: Route::Home,
            search: String::new(),
        }
    }

    #[test]
    fn an_open_action_moves_the_route() {
        let mut s = state("fieldbook");
        apply(&mut s, &json!({ "action": "open:collection:indices" }));
        assert_eq!(s.route, Route::Collection("indices".into()));
        apply(
            &mut s,
            &json!({ "action": "open:node:indices/amphibians" }),
        );
        assert_eq!(s.route, Route::Node("indices/amphibians".into()));
    }

    #[test]
    fn a_search_is_dropped_when_the_route_moves() {
        let mut s = state("fieldbook");
        apply(&mut s, &json!({ "action": "open:collection:journal" }));
        apply(
            &mut s,
            &json!({ "action": "search", "values": { "search": "culvert" } }),
        );
        assert_eq!(s.search, "culvert");
        apply(&mut s, &json!({ "action": "open:collection:indices" }));
        assert!(s.search.is_empty());
    }

    #[test]
    fn an_unknown_action_leaves_the_reader_where_they_were() {
        let mut s = state("atlas");
        apply(&mut s, &json!({ "action": "open:collection:venues" }));
        apply(&mut s, &json!({ "action": "no-such-verb" }));
        assert_eq!(s.route, Route::Collection("venues".into()));
    }

    #[test]
    fn the_document_version_moves_with_the_route_and_holds_still_otherwise() {
        let mut s = state("atlas");
        let before = s.document_version();
        // POSITIVE CONTROL: it can change.
        apply(&mut s, &json!({ "action": "open:collection:venues" }));
        let after = s.document_version();
        assert_ne!(before, after);
        // NEGATIVE CONTROL: it can hold still. A stamp that only ever moves
        // would drag a full refetch on every heartbeat and still pass a test
        // that checked only the first half.
        assert_eq!(after, s.document_version());
    }
}
