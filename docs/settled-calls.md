# Settled calls

Decisions the owner has made, with the reasoning that produced them. **One question, one owner.**
This file exists because the project is currently all decisions and no code, and a decision
without its reasoning gets re-litigated by the next person to read the repo.

Nothing here is reopened without the owner. If a call turns out wrong, the entry is amended in
place with the date and what killed it, so the file shows its own error rate.

---

## 2026-08-09 — Collaboration belongs to yggterm, not to ztlkasten

**Call:** ztlkasten does not build a collaboration layer. The primitive lives in yggterm.

**His reasoning:** *"I want the collaboration system simple as each yggterm client touching a
ssh remote is a 'collaborator'. This spec is honestly unbuilt and the collaboration should live
in the yggterm land. If yggterm can figure out simple collaboration like unix users, etc. then
every libyggterm app automatically becomes collaborative."*

**Consequences, and they are large:**

- The arbiter design previously drafted for ztlkasten — a host-resident daemon owning ordering,
  presence, leases and broadcast, with each client writing as its own uid so the kernel enforces
  permissions — **relocates to yggterm as a platform capability.** The thinking is not discarded;
  it changes address.
- ztlkasten **consumes** an arbiter; it does not own one.
- ⇒ The previous build order is void in its motivation. A kanban board was slated as the flagship
  *because it dogfooded the arbiter on the smallest possible write*. With the arbiter gone
  upstream there is no reason for a board to come first, and the build order should reorder
  around the actual purpose. See the spec.

**Load-bearing observation for whoever picks up the yggterm side:** the primitive this call asks
for is the same one yggterm's own constitution already demands — *two live viewers of one
session, with different window sizes*, rather than the read-only pinned shadow that currently
dodges the single-viewer assumption. **One solution, two payoffs.** It is already recorded there
as the highest-value load-bearing work in that project.

---

## 2026-08-09 — `emd-renderer` is the SSOT for markdown

**Call:** every markdown capability lands in `emd-renderer` (MPL-2.0, inside libyggterm), never
in an individual app.

**His reasoning:** *"emd-renderer is SSOT. All my software using markdown get auto upgraded and
is pipeline simplicity. Japanese Kaizen."*

**Consequences:** database views, calendars, board rendering, callouts, embeds and any future
block type are renderer work, not ztlkasten work. An app that grows its own markdown handling
breaks the property that makes this worth doing — improve the organ once, every consumer
improves.

---

## 2026-08-09 — Flatten: a folder boundary is enough

**Call:** one collection with sub-collections. No rigid vault walls.

**His reasoning:** *"I really do not like the vault rigid boundary and my iteration of the system
to main, main-extras (like debian packages) is more flowy I think. So folder boundary is enough
I guess. In short flatten."*

**Origin:** two sibling Obsidian vaults could not embed each other's artifacts, because Obsidian
anchors reference resolution at a vault root. A real archive had to choose between keeping notes
beside their evidence and keeping them in the graph, and the human absorbed the cost.

**Constraint that must survive the flattening:** some sub-collections must never enter a
publication path, and that separation has to be *declared and enforceable* rather than an
accident of which directory a file happens to sit in.

**Reframe worth keeping:** this is a **resolution-scope** problem, not a rendering one. The
decision is what a reference resolves *against* — note-relative, collection-relative, or a
declared set of roots — and only then how it renders.

---

## 2026-08-09 — Scope: journalling first, everything else is carried

**Call:** ztlkasten grows toward Notion's feature surface — boards, calendars, todo, agent
workflows — and is collaborative when users want it to be. But the design is judged on
journalling and long-form writing.

**His reasoning:** *"ztlkasten will grow to capture the features of notion … I use it for
journalling. There is stopping no one to use it for knowledge base work with agents too. Or
kanban/scrum/calendar with agents."*

⇒ Both framings are live and they are not in conflict, but **ordering matters**: features arrive
as the vault's own needs grow, and a feature that does not serve capture-and-retrieval does not
get to come first merely because it is easier to demo.
