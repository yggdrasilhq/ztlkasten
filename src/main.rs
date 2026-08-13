//! `kasten` — a corpus overview for a kasten-shaped collection.
//!
//! One binary, many corpora. Everything corpus-specific lives in a `kasten.toml`
//! at the corpus root; this program never learns a corpus's vocabulary, which is
//! what lets it ship in public while the corpora it reads stay private.
//!
//! The routes are addressable from the command line on purpose: a surface that
//! can only be inspected by looking at a GUI cannot be tested, and a test that
//! needs a desktop does not run.

mod corpus;
mod init;
mod launcher;
mod manifest;
mod osc;
mod schema;
mod server;

use anyhow::{bail, Context, Result};
use corpus::Corpus;
use manifest::Manifest;
use schema::Route;
use std::path::{Path, PathBuf};

const USAGE: &str = "\
kasten — a corpus overview

USAGE:
    kasten init  [--corpus <dir>]
    kasten serve [--corpus <dir>]
    kasten index [--corpus <dir>]
    kasten check [--corpus <dir>]
    kasten pane <doc|nav> [--route <route>] [--search <text>] [--corpus <dir>]

ROUTES:
    home                    the corpus overview
    collection:<id>         one collection
    node:<collection>/<slug>  one node

The corpus is the first of: --corpus, $KASTEN_CORPUS, or the nearest ancestor
directory of the working directory that holds a kasten.toml.
";

fn main() {
    if let Err(error) = run() {
        eprintln!("kasten: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional: Vec<String> = Vec::new();
    let mut corpus_arg: Option<PathBuf> = None;
    let mut route = "home".to_string();
    let mut search = String::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--corpus" => {
                corpus_arg = Some(PathBuf::from(next(&args, &mut i, "--corpus")?));
            }
            "--route" => route = next(&args, &mut i, "--route")?,
            "--search" => search = next(&args, &mut i, "--search")?,
            "-h" | "--help" | "help" => {
                print!("{USAGE}");
                return Ok(());
            }
            other if other.starts_with('-') => bail!("unknown option {other:?}\n\n{USAGE}"),
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    let Some(command) = positional.first().cloned() else {
        print!("{USAGE}");
        return Ok(());
    };

    // `init` runs before the manifest is located, because its whole job is the
    // case where there is not one yet. It writes nothing: the proposal goes to
    // stdout so a human reads it before it becomes the corpus's contract.
    if command == "init" {
        let root = match corpus_arg {
            Some(dir) => dir,
            None => std::env::current_dir()?,
        };
        print!("{}", init::propose(&root)?);
        return Ok(());
    }

    let manifest_path = locate(corpus_arg.as_deref())?;
    let corpus = Corpus::load(Manifest::load(&manifest_path)?)?;

    match command.as_str() {
        "serve" => serve(corpus, manifest_path),
        "index" => print_index(&corpus),
        "check" => print_check(&corpus, &manifest_path),
        "pane" => {
            let pane = positional.get(1).map(String::as_str).unwrap_or("doc");
            let route = Route::parse(&route);
            let value = match pane {
                "doc" => schema::document(&corpus, &route, &search),
                "nav" => schema::navigation(&corpus, &route),
                other => bail!("unknown pane {other:?} — expected doc or nav"),
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
            Ok(())
        }
        other => bail!("unknown command {other:?}\n\n{USAGE}"),
    }
}

fn next(args: &[String], i: &mut usize, flag: &str) -> Result<String> {
    *i += 1;
    args.get(*i)
        .cloned()
        .with_context(|| format!("{flag} needs a value"))
}

/// ⛔ The corpus is never guessed. When nothing declares one, the error says so
/// rather than defaulting to the working directory — an overview of the wrong
/// corpus is worse than no overview, and on a machine holding several it would
/// be indistinguishable from the right one at a glance.
fn locate(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        let path = if dir.is_dir() {
            dir.join("kasten.toml")
        } else {
            dir.to_path_buf()
        };
        if !path.is_file() {
            bail!("no kasten.toml at {}", path.display());
        }
        return Ok(path);
    }

    if let Some(env) = std::env::var_os("KASTEN_CORPUS") {
        let path = Path::new(&env).join("kasten.toml");
        if !path.is_file() {
            bail!("KASTEN_CORPUS is set but has no kasten.toml: {}", path.display());
        }
        return Ok(path);
    }

    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("kasten.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !dir.pop() {
            bail!("no kasten.toml here or in any parent — name one with --corpus or $KASTEN_CORPUS");
        }
    }
}

/// Hold the surface open: declare the panes, then re-declare on the heartbeat
/// cadence so the host can tell a live app from an abandoned one.
///
/// ⛔ The control URL is resolved ONCE, before the loop. Re-resolving it per
/// heartbeat is how an app spawns a forwarding tunnel every few seconds — a
/// leak that looks like nothing at all until the machine runs out of sockets.
fn serve(corpus: Corpus, manifest_path: PathBuf) -> Result<()> {
    let session = std::env::var("YGGTERM_SESSION_ID").unwrap_or_default();
    if session.is_empty() {
        // Not fatal: the schemas are still served and still fetchable, which is
        // exactly what the tests and a headless check need. What is missing is
        // a host to declare TO, and saying so beats appearing to work.
        eprintln!(
            "kasten: no YGGTERM_SESSION_ID — serving the schema, but no surface will open. \
             Run this inside a yggterm terminal to see it."
        );
    }
    launcher::write_best_effort();

    // Retire the declaration on the way out. An unswept contribution does
    // expire on its own, so this is not correctness — it is the difference
    // between a surface that closes when the reader closes it and one that
    // lingers for a timeout they can see and cannot explain.
    if !session.is_empty() {
        let session = session.clone();
        let _ = ctrlc::set_handler(move || {
            osc::close(&session);
            std::process::exit(0);
        });
    }

    let name = corpus.manifest.name.clone();
    let server = server::spawn(server::State {
        manifest_path,
        corpus,
        route: Route::Home,
        search: String::new(),
    })?;
    let control = server.url.clone();
    println!("kasten: {name} on {control}");

    let mut last = String::new();
    loop {
        let version = {
            let mut state = server.state.lock().unwrap();
            state.reload();
            state.document_version()
        };
        if !session.is_empty() {
            osc::declare(&session, &control, &name, &version);
        }
        if version != last {
            last = version;
        }
        std::thread::sleep(std::time::Duration::from_secs(4));
    }
}

fn print_index(corpus: &Corpus) -> Result<()> {
    println!("{}", corpus.manifest.name);
    for c in &corpus.manifest.collections {
        let count = corpus.count(&c.id);
        let publish = if c.publish { "" } else { "  [never published]" };
        println!("  {:<16} {:>4} {}{}", c.id, count, c.label, publish);
    }
    let wanted = corpus.unresolved();
    if !wanted.is_empty() {
        println!("  wanted: {}", wanted.join(", "));
    }
    Ok(())
}

/// What a manifest can get wrong that only the corpus can reveal: a collection
/// whose directory is not there, and a link with nothing behind it. Both are
/// reported, neither is fatal — a corpus is allowed to be mid-growth, and a
/// checker that refuses a working corpus gets switched off.
fn print_check(corpus: &Corpus, manifest_path: &Path) -> Result<()> {
    println!("manifest: {}", manifest_path.display());
    // The root is printed because the commonest way to be confused by this
    // program is to be looking at a corpus other than the one you meant.
    println!("root:     {}", corpus.manifest.root.display());
    let mut notes = 0;
    for c in &corpus.manifest.collections {
        if !c.path.is_dir() {
            println!("  ⚠ {}: no directory at {}", c.id, c.path.display());
            notes += 1;
        } else if corpus.count(&c.id) == 0 {
            println!("  · {}: directory exists, no nodes yet", c.id);
        }
    }
    for slug in corpus.unresolved() {
        println!("  · wanted: {slug}");
        notes += 1;
    }
    println!(
        "  {} node{} across {} collection{}{}",
        corpus.nodes.len(),
        if corpus.nodes.len() == 1 { "" } else { "s" },
        corpus.manifest.collections.len(),
        if corpus.manifest.collections.len() == 1 { "" } else { "s" },
        if notes == 0 { " — nothing to report" } else { "" },
    );
    Ok(())
}
