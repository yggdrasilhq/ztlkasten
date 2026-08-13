# `kasten.toml` — the corpus manifest

**Status:** implemented 2026-08-13, and this document owns it. The parser is
`src/manifest.rs`; where the two disagree, one of them is a bug and this file
says which behaviour was intended.

---

## What this file is for

The overview app ships once and reads many corpora. Some of those corpora are
private and their vocabulary — what their collections are called, what their
nodes are — is itself information about their owner. So **the engine never
learns a corpus's vocabulary. The corpus declares its own.**

⇒ `kasten.toml` sits at a corpus root and is the only thing that differs between
one corpus and another. The engine is public; a manifest may live anywhere,
including a private repository the engine has never seen.

**This is a structural guarantee, not a rule someone has to remember.** There is
no place in the engine where a corpus name could be written down, because there
is no code path that names one. A default that happened to match one corpus
would be that corpus's vocabulary smuggled into the engine, and every later
corpus would inherit a wrong answer that looked right — which is why the
defaults below are all derived from the *shape* a node has, never from any
particular corpus's habits.

## Minimum manifest

```toml
[corpus]
name = "Fieldbook"

[[collection]]
id = "journal"
node = "note"
```

Everything else has a default. A collection's `label` defaults to its `id`, its
`path` to a directory of the same name under the root, and its `title` to
whichever source suits its node shape.

## `kasten init` — a proposal, not a decision

Writing a manifest by hand is a chore assembled from primitives: list the
directories, work out which hold prose and which hold facts, guess where the
titles live, get one wrong, notice weeks later. `kasten init` does it the same
way every time and prints the result to stdout.

**What it will not do:**

- **overwrite an existing manifest.** A generator that replaced a hand-tuned
  file would make manifests untrustworthy exactly where it matters most — on a
  corpus nobody wants to re-check.
- **invent a source it did not observe.** A date source is emitted only when
  dated filenames or a date key were actually seen.
- **silently resolve a mixed directory.** A directory holding both prose nodes
  and record nodes gets a `⚠ MIXED` comment, because only one shape can be
  declared and the other becomes invisible.
- **drop a directory it skipped.** Candidates that held nothing are listed as
  comments, so their absence is a recorded fact rather than an oversight.

**What it cannot know, by construction, and therefore leaves for a human:**

| | why |
|---|---|
| `name` | it uses the directory name; what a corpus is *called* is not on disk |
| `publish` | a policy declaration, not an observable property |
| `label` | directory names are rarely what you would call the thing out loud |

⇒ Measured against a corpus whose manifest was written by hand, the proposal
recovered every collection, every node shape, and every title, date and status
source. The two it did not recover are precisely the two in that table.

## `[corpus]`

| key | meaning |
|---|---|
| `name` | Display name. Appears in the chrome and the footer. Required. |
| `root` | Where the collections live, relative to the manifest. Defaults to the manifest's own directory. |

## `[capture]` — the first hot path

```toml
[capture]
collection = "journal"
```

**One key, and it is the only decision the writer ever makes about capture** —
made once, in this file, rather than every time a thought arrives.

⛔ **Everything else is DERIVED, not restated.** The file name, the heading and
the ordering all come from what the target collection already declares about
itself. A corpus that said `date = "filename"` has already said how its entries
are named, and a second place to say it is a second place to disagree.

**What a capture costs the writer:** one command, no prompts, no title, no
folder, no tag.

```sh
kasten capture the four-degree rule held again at the [[culvert]]
```

- Lands in `<collection>/<today>.md`, creating it on the day's first thought and
  joining it thereafter — **a day is one entry, not one per thought**.
- A new entry opens with `# <date>`, which is what a `note` collection reads its
  title from. **The entry titles itself.**
- Every thought gets a `## HH:MM` marker. It costs no keystrokes and no
  decisions — the writer never types it — and it is the only way to recover the
  order of a day's thinking months later. The flow budget prices rules in
  decisions and keystrokes on the hot path; this adds neither.
- A `[[link]]` typed mid-sentence is indexed with **no second gesture**, and an
  unwritten target becomes a *Wanted* item — a to-write list the writer produced
  by writing.
- The collection's directory is created if absent. Failing because a folder did
  not exist yet is exactly the bureaucratic resistance this program is judged on.

**Refused at manifest LOAD**, so a corpus can never be configured to swallow
thoughts into somewhere that will not show them:

- a `[capture]` naming a collection the corpus does not declare;
- a collection of **records** — a captured thought is prose, and filing it as
  facts puts it where nothing reads it;
- a collection whose date does **not** come from the filename — capture writes a
  dated file, so the collection has to be able to read a date back out of one.

**Absent `[capture]` ⇒ the corpus is a reader.** The app offers no capture box at
all rather than guessing a destination for someone's writing.

## `[overview]`

| key | meaning |
|---|---|
| `recent` | How many recently-dated nodes the overview shows. Default 8. |

## `[[collection]]`

One per collection, in the order they should appear.

| key | meaning | default |
|---|---|---|
| `id` | Addresses the collection in a route. Must be unique. | required |
| `label` | What the reader sees. | the `id` |
| `path` | Directory, relative to the root. | the `id` |
| `node` | `note` or `record` — see below. | required |
| `entry` | For a directory node, the file inside it holding the facts. | `index.toml` |
| `title` | Where a node's title is read from. | `heading` for a note, `slug` for a record |
| `date` | Where a node's date is read from. | none |
| `status` | Where a node's status is read from. | none |
| `order` | `date-desc`, `date-asc`, `title`, `slug`. | `title` |
| `publish` | `false` ⇒ this collection may never enter a publication path. | `true` |

### Node shapes

- **`note`** — one prose file per node. The slug is the file stem. This is the
  journalling shape: a note is a file, and so is every grouping that organises
  notes, which is the rule the whole system rests on.
- **`record`** — structured facts plus optional prose. Either a `<slug>.toml`
  with an optional `<slug>.md` beside it, or a directory `<slug>/` holding a
  facts file and optional prose of the same stem. **Both forms are supported
  because both occur**: a corpus grows directory nodes the moment a node needs
  to carry attachments, and it should not have to migrate the ones that do not.

⚠ **A directory node's facts file is named by the corpus, not by this spec.**
Real corpora call it after the node kind — a matter's directory holds
`matter.toml`, a year's holds `year.toml` — rather than adopting one universal
name. So `entry` declares it. A hard-coded list of likely filenames was the
obvious alternative and is the wrong one: it works on the corpus it was written
against and **silently reads zero nodes** on the next, which looks identical to
an empty collection. This was found by running `init` against a real corpus and
watching two populated collections be reported as holding nothing.

`init` detects it by observation: a directory that holds exactly one top-level
`.toml` names it, and the name most of a collection's directories agree on is
proposed. A directory holding two is not treated as a node at all — which one
is the node would be a guess, and a wrong guess parses fine and shows the wrong
title forever.

### Sources

A source says where a field is read from, so that two corpora with entirely
different conventions need no code between them.

| source | reads |
|---|---|
| `heading` | The first `# ` heading of the prose. |
| `frontmatter:<key>` | A key in the prose's YAML frontmatter. |
| `facts:<key>` | A key in the facts file. Dotted keys walk nested tables. |
| `filename` | A leading `YYYY-MM-DD` in the file name. |
| `slug` | The slug, with separators as spaces. |
| `mtime` | Filesystem modification time. |

**A source that finds nothing yields nothing.** `filename` on a file with no
leading date returns no date rather than a guess — a date the engine invented
would sort a listing and never be questioned, which is worse than an absent one
the reader can see is absent.

## What the manifest is checked for at load

Refused, with the field named:

- an unknown node shape, source or order;
- a duplicate collection `id`;
- **an order by date where no date source is declared.** The list could not sort
  and would quietly be in declaration order while claiming to be chronological.
- a manifest with no collections at all.

Not refused:

- **a collection whose directory does not exist.** It is empty, not fatal. A
  manifest describes a shape a corpus may take, and a corpus is allowed not to
  have started a collection yet. `kasten check` reports it.

## `publish = false`

The spec requires that some sub-collections never enter a publication path, and
that the separation be **declared and enforceable rather than an accident of
which directory something sits in**. This key is that declaration. It is carried
to the surface — the collection's row says so — so the property is visible to a
reader rather than being something they are expected to remember.

⚠ Honest limit as of 2026-08-13: nothing yet *consumes* the flag beyond
displaying it, because there is no publication path in this repo to enforce it
against. The flag exists first so that the path, when it is built, has something
to refuse. A publication path that had to invent the concept later would define
it to suit itself.

## Links

`[[target]]` and `[[target|label]]` in prose. A target is a slug and it resolves
**corpus-wide**, regardless of which collection holds it — the settled call is
that a folder boundary is enough, and reference resolution anchored at a
container root is exactly what this design drops.

A link whose target does not exist is **not an error**. In a system where a tag
is a note, an unwritten target is a note that has been called for and not yet
written, so the overview lists it under *Wanted*. It is a to-write list the
writer produced by writing, at a cost of zero extra keystrokes — which is the
only kind of retrieval affordance the design value permits.

## Determinism

Every listing is sorted, and every comparator ends in the slug so the order is
total. Directory order is whatever the filesystem returns; a corpus that renders
differently on two machines is a bug rather than a cosmetic difference, because
the reader learns where things are and then the shelf moves.
