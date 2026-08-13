# ztlkasten — the specification

**Status:** seeded 2026-08-09. Sections marked ⛏ are unwritten and are the next work.

This document owns **how ztlkasten must behave**. Reasoning and history live in the owner's
discussion notes; decisions and their justifications live in [`settled-calls.md`](settled-calls.md).
When they disagree, fix the layer that drifted.

---

## 1. What ztlkasten is

**A set of rules for organising a markdown vault, and an application that makes those rules
first-class.** Roam-like, with one difference doing most of the work:

> **A tag is a note.**

Not only tags. Indices, characters, discussions and every other kind of grouping are ordinary
files, so the vocabulary used to organise is made of the same material as the things organised.
There is no second class of object, no database rows, no metadata store — a grouping is a file
you can open, write in, link from, and journal inside.

**Built for journalling and long-form writing first.** Everything else the vault can carry —
knowledge bases, boards, calendars, agent workflows — is carried, not the reason it exists.

## 2. The design value: flow

> *"Flow is important, as resistance and bureaucraticness break the writing OR reading flow."*

This is the only value the spec ranks above the others, and it needs to be testable rather than
felt. **Two hot paths, and both stay cheap:**

1. **Capture a thought.**
2. **Find a thing again.**

⇒ **Every rule in this document carries a price in decisions and keystrokes on those two paths.**
A rule that adds a decision to either needs a stated reason next to it. A feature that cannot
justify its cost on both paths is a feature for some other program.

This converts flow from taste into a budget, which is the only form in which a spec can defend
it.

## 3. Architecture: ztlkasten is thin, deliberately

| Capability | Owner | Status |
|---|---|---|
| markdown parse + render | `emd-renderer` (libyggterm, MPL-2.0) | exists, v0.3.0 |
| app shell, panes, widgets | `yggui` (libyggterm) | exists |
| multi-client collaboration | **yggterm** | not built; see settled-calls |
| vault rules, surfaces, views | **this repo** | ⛏ |
| the corpus manifest | **this repo** | [`spec-manifest.md`](spec-manifest.md), implemented |
| the overview surface | **this repo** | `kasten`, reading + retrieval only |

**The overview is Tier A of the platform's app-architecture contract**: the host paints the
content with widgets it already has, and this repo ships no user-interface code. The question
that decides a tier is *who must paint the pixels, and why* — for collections, rows, prose,
counts and a search box the answer is the host, and reaching for a foreign engine to draw one of
them would serve this app and charge every app on the platform forever.

**The rule that generates this table: if a capability could serve another app, it belongs
upstream.** Improve the organ once and every consumer improves — the reason `emd-renderer` is
the single source of truth for markdown, and the reason collaboration is not ztlkasten's to
invent.

### 3.1 Not in scope

- Authentication, accounts, sharing UI. The OS does this: identity is the unix user, sharing is
  file permissions and groups, access is ssh.
- Realtime character-level co-editing of prose.
- A pixel-perfect WYSIWYG block editor.
- Non-file-backed records. If it is not a note, it is not a record.

## 4. The vault model ⛏

**This section is written by extraction, not by design.** The rules already exist and have been
in daily use for months in an Obsidian vault; that vault is the specification, and the work is to
read it and write down what is true.

⛔ **The vault is a private journal.** Describe the *structure* and invent every example.
No real note titles, no real people, no paths from the owner's machines, no host names. A public
repo must carry the shape of the system and none of its contents.

To be written:

- **Node kinds and the folder vocabulary** — what each top-level folder means, and what makes
  something belong in one rather than another.
- **Tag-as-note mechanics** — how a grouping file is created, what it holds, and what happens
  when it is referenced before it exists.
- **Identity and disambiguation** — ⚠ if characters are notes, two real people with the same
  name need two nodes. Naming, aliasing and merge behaviour are unsolved and this is the first
  concrete rule gap found in practice.
- **The journal entry** — how an entry enters, where it lands, what it links to automatically,
  and what the capture path costs in keystrokes.
- **Retrieval** — the second hot path. How a thing is found again months later, and what the
  index files do for it.
- **Artifact references** — see §5.

## 5. Reference resolution

**Flattened: one collection with sub-collections.** No rigid vault walls.

The failure this fixes: reference resolution anchored at a container root means a note cannot
embed an artifact stored under a sibling container, which forces the writer to choose between
keeping notes beside their evidence and keeping them in the graph.

⇒ **A reference resolves regardless of which sub-collection the target lives under.** The open
design question is *what* it resolves against — note-relative, collection-relative, or a declared
set of roots — and that decision precedes any rendering work.

**Constraint:** some sub-collections must never enter a publication path. That must be a declared
and enforceable property, not an accident of directory layout. ⛏ Mechanism unwritten.

## 6. Build order ⛏

Not yet written, and deliberately not inherited. The previous ordering led with a kanban board
because a board dogfooded the collaboration arbiter on the smallest possible write; with
collaboration moved to yggterm that rationale is gone.

**The ordering principle:** earn the two hot paths first. A feature that does not serve capture
or retrieval does not come first merely because it demos well.
