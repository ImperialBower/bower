# Configuration

Two files at a book's root. One is written by hand; one is generated and
reviewed.

* [bower.toml](bower-toml.md) - epoch, identity, and per-repo remotes, templates, commands and link templates. Unknown fields are a load-time error.
* [bower.lock](bower-lock.md) - the resolved plan as text, checked in so reordering shows up in review. Written by the kernel, never parsed.
