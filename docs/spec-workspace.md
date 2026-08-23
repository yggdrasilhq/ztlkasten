# Kasten workspace

The workspace is the host-local set of roots Kasten reads together. It exists
to remove the vault boundary from reference resolution without changing the
files that another editor already uses.

## Configuration

`~/.yggterm/kasten/config.toml` is owned by the local yggterm installation:

```toml
version = 1
master = ["/home/example/notes", "/mnt/example/archive"]
```

There may be any number of master folders. Each master may itself be an
Obsidian vault, or may contain several direct child vaults. A vault is detected
structurally by its `.obsidian` directory. The configuration is deliberately
outside every vault.

If the file is absent, `kasten serve` runs the same guided setup as bare
`kasten init`. For automation, repeat `--master`:

```sh
kasten init --master ~/notes --master /mnt/example/archive
```

The command validates and adds the folders to the host-local configuration;
repeating a folder is idempotent.
It never writes a `kasten.toml`, index, cache, or metadata file into a vault.

## Discovery and navigation

The vault root and each immediate child directory containing Markdown files
directly are note collections. Hidden directories and deeper nested trees are
excluded. This preserves the settled distinction: flat top-level note folders
are collections, while nested trees are artifact stores rather than shelves in
the sidebar.

The sidebar first lists vaults for rapid switching, then groups the discovered
collections under their vault labels. Vault and collection routes remain
addressable through the same generic libyggterm schema used by the rest of the
application.

## Cross-vault references

All configured vaults form one resolution scope. `[[name]]`,
`[[folder/name]]`, and `[[name#heading]]` resolve against the note filename
across that entire scope. Backlinks use the same scope. If several notes have
the same filename, Kasten presents every match with its vault and collection;
it does not guess which one the writer meant.
