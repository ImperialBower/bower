# Operations

How the project is built, gated, and shipped.

* [make ayce — the gate](make-ayce.md) - the default target and the whole pre-push sweep, plus the rule that keeps its contract fixed.
* [CI — the fast lane and the slow lane](ci-lanes.md) - five parallel jobs on every PR; the tool-dependent lanes on `main` only.
* [Releases and the site branch](releases-and-site.md) - shipping the repository, its rendered site, and its downloads in one command.
