# CLAUDE.md

## ⛔⛔ PRIVACY — this repo describes a private journal, and it is destined to be PUBLIC

**Run `scripts/install-hooks.sh` once per clone**, then `scripts/check-privacy.sh` before every
commit. It scans tracked *and* untracked files, because the moment this guard exists to cover is
the moment just before `git add`.

⛔ **The hook is not optional and it does not travel.** Git hooks live in `.git/`, so a fresh
clone has no guard at all until the installer is run — and the guard was hand-run only until
2026-08-13, which works exactly as long as whoever is working remembers it. **An agent's
discipline resets every session; a hook's does not.**

⚠ **Prove an installed hook with a REAL PUSH, both directions.** A hook that is present,
executable and syntactically valid is indistinguishable on disk from one that works, and a broken
one can print the guard's own output while exiting non-zero — which git reads as *refuse* and
which looks exactly like the guard doing its job. A planted leak must be refused **and** a clean
push must succeed: a hook that refuses everything passes the first test, and one that does
nothing passes the second.

ztlkasten is a set of rules extracted from a working vault. **That vault is the owner's personal
journal.** It holds notes about real people, family, health, money, legal matters and private
work. The specification must carry the *shape* of the system and **none of its contents**.

⛔ **The leak vector is never a secret. It is an agent writing a REAL example into a document
because a real example was in front of it.** Every instance in the sibling repo arrived that way,
twice, and the second time was an agent that had just read the rule.

⇒ **INVENT every example.** A folder name, a note title, a person, a path, a tag, a filename.
`notes/`, `evidence/set-01/figure.png`, `/home/user/vault`, `characters/alice.md`. An invented
example specifies exactly as well as a real one.
⇒ **Describe structure, never content.** "A character note holds one person and is linked from
every note that mentions them" is the spec. Which people exist is not.
⇒ **A war story cites the SYMPTOM, never the case.** "Reference resolution anchored at a
container root cannot cross containers" — not which containers, whose, or what was in them.
⇒ **Never name the host, the mount, the fleet, or any private store.**

If you cannot make the point without a real example, the point is not yet abstract enough to be
a specification.

## What this project is, before you design anything for it

**ztlkasten is a journalling system first.** Roam-like, with one rule doing most of the work:
**a tag is a note.** Indices, characters, discussions and every other grouping are ordinary
files, so the vocabulary is made of the same material as the things it organises.

It is the successor to org-mode and to a hosted notes app in a long journalling lineage, and the
owner intends it to be his **last** system, because he controls it from the ground up. Judge every
proposal against *does this help him write*, never against feature parity with Notion, Roam or
Obsidian.

## ⭐ FLOW is the ranked value, and it is a budget rather than a taste

> *"Flow is important, as resistance and bureaucraticness break the writing OR reading flow."*

**Two hot paths stay cheap: capture a thought, and find a thing again.** Every rule carries a
price in decisions and keystrokes on those two paths, and a rule that adds a decision to either
needs its reason written next to it.

⇒ **This applies to the repo's own process too.** Do not import a six-organ documentation
apparatus into a project that has no code yet. Bureaucracy about a system whose purpose is the
absence of bureaucracy is a self-inflicted wound. Structure earns its way in when there is
something for it to hold.

## ⛔ ztlkasten is THIN — if a capability could serve another app, it belongs upstream

| Capability | Owner |
|---|---|
| markdown parse and render | `emd-renderer` in libyggterm — **the SSOT for markdown**, ecosystem-wide |
| app shell, panes, widgets | `yggui` in libyggterm |
| multi-client collaboration | **yggterm** — a client reaching an ssh remote *is* a collaborator |
| vault rules, surfaces, views | this repo, and only this |

A markdown feature implemented here instead of in `emd-renderer` breaks the property that makes
the whole pipeline worth having: improve the organ once, every consumer improves. Same for
concurrency — see `docs/settled-calls.md`.

## The spec is written by EXTRACTION, not design

The rules have been in daily use for months. **The working vault is the specification.** The task
is to read it and write down what is true, which is cheap, checkable, and produces a spec that
describes something real. It is also the task with the highest leak risk — see the privacy rule
above, which is why it is first in this file.

## Single source of truth

`docs/spec-ztlkasten.md` owns **how it must behave**. `docs/settled-calls.md` owns **what the
owner decided and why**. Never answer a question from a file that does not own it, and if the two
disagree, fix the layer that drifted.
