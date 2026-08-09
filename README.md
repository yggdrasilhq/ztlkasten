# ztlkasten

A vault-shaped knowledge and journalling application for the Yggdrasil ecosystem.

ztlkasten is a **set of rules for organising a markdown vault**, and an application that makes
those rules first-class. Roam-like, with one difference that does most of the work: **a tag is a
note**. Not only tags — indices, characters, discussions and every other kind of grouping are
ordinary files, so the vocabulary you organise with is made of the same material as the things
you organise.

It is built for **journalling and long-form writing first**. Knowledge-base work, boards,
calendars and agent-driven workflows are things the same vault can carry, not the reason it
exists.

## What it is not

It is not a Notion clone, and it deliberately does not rebuild the two thirds of Notion that
exist only because Notion refused to be a unix program:

- **Identity** is the unix user. No accounts, no SSO, no invite flow.
- **Sharing** is file permissions and unix groups.
- **Access** is ssh. Reaching the host that holds the vault *is* the authorization.
- **Collaboration** is not ztlkasten's to build — see below.

## Where the parts live

ztlkasten is a thin consumer of two platform organs, and that is the architecture, not an
implementation detail:

| Capability | Owner | Why |
|---|---|---|
| Markdown parsing and rendering | **`emd-renderer`** in [libyggterm](https://github.com/yggdrasilhq/libyggterm) | one renderer, so every markdown surface in the ecosystem improves at once |
| Multi-client collaboration | **[yggterm](https://github.com/yggdrasilhq/yggterm)** | a yggterm client reaching an ssh remote *is* a collaborator; solve it once and every libyggterm app inherits it |
| Application shell, panes, widgets | **`yggui`** in libyggterm | the app-tier contract |
| Vault rules, surfaces, views | **this repo** | the only part that is ztlkasten's own |

If a capability could serve another app, it belongs upstream. That rule is the point.

## Status

Early. The specification is being written before the code, and the specification is being
*extracted from a working vault* rather than designed in the abstract — the rules have been in
daily use for months in Obsidian, which ztlkasten is intended to replace.

See [`docs/`](docs/).

## Licence

GPL-3.0-or-later for code. Documentation under CC-BY-SA-4.0.

ztlkasten links `libyggterm`, which is MPL-2.0 — permitted under MPL §3.3, and libyggterm
carries no Exhibit B.
