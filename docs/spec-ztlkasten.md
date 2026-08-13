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

### ✅ Node kinds and the folder vocabulary — extracted 2026-08-13

Measured against a vault in daily use, by counting structure rather than reading content.
**Every example below is invented; what is real is the shape and the numbers.**

**A note kind is a FLAT folder of markdown files. A nested tree is an artifact store, not a
node collection.** The split is clean and it is the rule that decides where something belongs:

| | shape | holds |
|---|---|---|
| **node collection** | flat, dozens-to-hundreds of `.md`, **no subdirectories** | `characters/`, `discussions/`, `tags/`, `indexes/`, `guides/` |
| **artifact store** | nested subdirectories, few or no `.md` at the top | a manuscript tree, a publication output tree, an assets tree |

⇒ This is why the manifest's collection model works unchanged on a vault: **a collection is a
flat folder of nodes**, and the nested trees are not collections at all. A vault that mixes them
in one directory has a folder that is two things.

**The largest node kinds are the ones used for organising, not the ones being organised.** In the
measured vault the tag folder and the character folder are each larger than the discussion
folder. **The vocabulary outweighs the material**, which is what "a tag is a note" produces in
practice and is the strongest evidence the rule is load-bearing rather than decorative.

**Sub-collections are peers, not a hierarchy.** The vault is several sibling collections — a
primary writing collection, a bulk/working collection, and a generated collection — each with
its own `indexes/` and `templates/`. **An index is per-collection**, not global. This is the
`debian packages` framing that produced the flatten call, and it survives it: separation by
folder, no wall.

### ✅ Link syntax — measured, and it settles a design question

Counted across the primary collection. Only the shapes are reported; no targets were read.

| shape | share | reading |
|---|---|---|
| **bare `[[name]]`** | **~98%** | the overwhelming default |
| pathed `[[folder/name]]` | ~1.8% | rare, and a workaround where it appears |
| aliased `[[name\|label]]` | ~0.3% | almost never |
| heading `[[name#heading]]` | **0** | never used, not once |
| embed `![[name]]` | 3 total | effectively unused |
| `#hashtag` | ~6% of link volume | present, but the link is the primary gesture |

⇒ **A LINK IS A BARE NAME, and the system resolves it.** This is not a preference to be argued
about — 98% of a working vault's links carry no path at all, which is the flatten call arriving
as data. **A resolver that cannot find a target by bare name across the whole collection is
useless here; one that lacks aliases and heading-links loses under half a percent.** That is a
priority ordering for renderer work derived from use rather than from taste.

⚠ **The embed count is 3, and it admits two readings.** Cross-collection artifact embedding was
the origin story of the flatten decision — a note could not embed evidence stored under a
sibling container. Three embeds is consistent with *the containment made it impractical* and
equally consistent with *nobody wanted it*. **The number does not distinguish them**, and it
should not be quoted as if it did.

**Frontmatter is near-universal** — present on ~98% of notes. So `frontmatter:<key>` is the
normal place a vault-shaped corpus keeps a title, a date or a status, not an edge case.

Still to be written:
- **Tag-as-note mechanics** — how a grouping file is created, what it holds, and what happens
  when it is referenced before it exists.
- **Identity and disambiguation** — ⚠ if characters are notes, two real people with the same
  name need two nodes. Naming, aliasing and merge behaviour are unsolved and this is the first
  concrete rule gap found in practice.
- **The journal entry** — ✅ the ENTRY half is now built and specified; see
  [`spec-manifest.md`](spec-manifest.md) §capture. A thought enters with one command and no
  decisions: it lands in today's dated entry in the declared collection, creating it on the
  first thought of the day and joining it thereafter, under an automatic time marker. The
  entry titles itself from its date, so the writer never names one. ⛏ Still unwritten: what an
  entry should link to *automatically*, which is a rule about the system rather than a
  mechanism — today a link costs the writer the `[[…]]` they type and nothing else.
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
