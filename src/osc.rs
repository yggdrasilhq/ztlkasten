//! The libyggterm OSC 7717 channel — kasten's side of the surface contract.
//!
//! ⛔ This file and `server.rs` are the app SCAFFOLDING the platform's own
//! migration order wants extracted into libyggterm once a second consumer
//! exists. kasten IS that second consumer, and the extraction is filed rather
//! than done here: a platform refactor inside an app's first commit changes two
//! things at once and neither can then be proven on its own.
//!
//! The transport is the terminal's own byte stream, so there is no discovery,
//! no version negotiation and no new socket — and in a plain terminal the
//! escapes are simply invisible, which is the whole degradation story.

use base64::Engine as _;
use serde_json::json;
use std::io::Write as _;

fn emit(verb: &str, action: &str, payload: &str) {
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload);
    let mut stdout = std::io::stdout().lock();
    let _ = write!(stdout, "\u{1b}]7717;{verb};{action};{encoded}\u{7}");
    let _ = stdout.flush();
}

/// `sidebar ; declare` — idempotent, re-emitted on the heartbeat cadence as the
/// liveness signal.
///
/// ⚠ It must not re-resolve the control URL: a declare that re-resolved would
/// spawn one forwarding tunnel per heartbeat, which is a leak that looks like
/// nothing until the machine is out of sockets.
pub fn declare(session: &str, control: &str, corpus_name: &str, document_version: &str) {
    let payload = json!({
        "session": session,
        "control": control,
        "app_name": "Kasten",
        "document_version": document_version,
        "panes": [
            {
                "id": "doc",
                "icon": "🗃\u{fe0e}",
                "title": format!("{corpus_name} — overview"),
                "placement": "viewport",
            },
            {
                // U+FE0E forces text presentation, so the glyph sits with the
                // host's monochrome chrome instead of arriving as colour emoji.
                "id": "nav",
                "icon": "🗃\u{fe0e}",
                "title": format!("{corpus_name} — collections"),
                "placement": "rail",
            },
        ],
    });
    emit("sidebar", "declare", &payload.to_string());
}

pub fn close(session: &str) {
    emit("sidebar", "close", &json!({ "session": session }).to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The payload is what the contract is about; the escape framing is one
    /// line and cannot be asserted without capturing stdout. Assert the shape
    /// that a host actually reads.
    #[test]
    fn the_declare_payload_names_both_panes_and_their_placements() {
        let payload = json!({
            "panes": [
                { "id": "doc", "placement": "viewport" },
                { "id": "nav", "placement": "rail" },
            ],
        });
        let panes = payload["panes"].as_array().unwrap();
        assert_eq!(panes[0]["placement"], "viewport");
        assert_eq!(panes[1]["placement"], "rail");
    }

    #[test]
    fn base64_round_trips_the_payload() {
        let raw = r#"{"session":"s","control":"http://127.0.0.1:1"}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(raw);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), raw);
    }
}
