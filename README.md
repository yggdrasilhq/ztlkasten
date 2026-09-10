# ztlkasten

> ⭐ **FROZEN 2026-09-05 — read this before building here.** The app surface
> is superseded: the owner's graph tooling consolidates into
> **`msggraph/lobemanager/`** (shared verbs: brief, lint, graduate) and
> **ymacs** (the Common Lisp emacs on libyggterm) as the one interactive
> surface over all five graph lobes — a kasten-mode there, not an app here.
> **The RULES this repo encodes are NOT deprecated** — a tag is a note, the
> vault vocabulary is made of the same material as the corpus: they live in
> the corpus contracts themselves (`kastengraph/AGENTS.md`,
> `vault-graph-check.py`) and in the `kasten.toml` manifest format, which
> stays the one coupling between a corpus and any overview tool. New work:
> ymacs packages and lobemanager verbs, not this repo. This repository is
> the spec and the archive of those rules.

A vault-shaped knowledge and journalling application for the Yggdrasil ecosystem.

ztlkasten is a **set of rules for organising a markdown vault**, and an application that makes
those rules first-class. Roam-like, with one difference that does most of the work: **a tag is a
note**. Not only tags — indices, characters, discussions and every other kind of grouping are
ordinary files, so the vocabulary you organise with is made of the same material as the things
you organise.

It is built for **journalling and long-form writing first**. Knowledge-base work, boards,
calendars and agent-driven workflows are things the same vault can carry, not the reason it
exists.

## Archived installation

The `@ygghq/kasten` app package is deprecated and has been removed from the
yggterm fleet. Do not install it with ynpm or ynpx. Use ymacs for the
interactive editor surface and msggraph/lobemanager for graph operations.
Existing fleet installations can be removed with:

```sh
ynpm remove @ygghq/kasten
```

## What it is not

It is not a Notion clone, and it deliberately does not rebuild the two thirds of Notion that
exist only because Notion refused to be a unix program:

- **Identity** is the unix user. No accounts, no SSO, no invite flow.
- **Sharing** is file permissions and unix groups.
- **Access** is ssh. Reaching the host that holds the vault *is* the authorization.
- **Collaboration** is not ztlkasten's to build — see below.
