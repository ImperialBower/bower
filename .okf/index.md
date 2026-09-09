---
okf_version: '0.1'
---

# Bower — book-driven repository generation and publishing

Annotated code blocks in a book's markdown are the single source of truth. Bower
replays them into deterministic git repositories — one commit per teaching step
— and links page and code to each other in both directions.

# Start here

* [What Bower is](overview.md) - the inversion: the repo becomes a build artifact of the book.
* [Getting started — orienting in this repository](getting-started.md) - the shortest path to a working mental model, and the daily loop.

# The domain

* [The domain model](model/) - the vocabulary the kernel thinks in: directives, steps, plans, tree states, display markers, expectations, errors.
* [Architecture](architecture/) - the pipeline, the domain-kernel pattern, the four crates, and the invariants each test suite proves.

# Using it

* [The command surface](commands/) - `plan`, `build`, `verify`, `status`, `publish`, `push`.
* [Configuration](config/) - `bower.toml` written by hand, `bower.lock` generated and reviewed.
* [The books](books/) - the sample fixture, and the real book in flight.
* [Operations](operations/) - the `make ayce` gate, the CI lanes, releases and the site branch.

# Context

* [Decisions](decisions/) - questions the project has settled, with dates and reasoning.
* [Roadmap](roadmap/) - what is built, what is designed, and what is still open.
* [References](references/) - the design spec and the rest of the in-repo documentation.
