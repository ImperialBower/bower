# bower-ipynb-poc — playable chapters on the spark4 playbook

Proof of concept for Bower spec § 15 (*the notebook target*): one annotated
book chapter → one Jupyter notebook per chapter, prose as markdown cells, Rust
as read-only line-anchored views of the generated repo, and **play cells** as
live Python against a PyO3 wheel of the chapter's code. Execution surface is
`pkcore.py` running inside the spark4 image.

```
bower-ipynb-poc/
├── book/
│   ├── bower.toml              # repo config + [repos.failers.notebook] wheel pin
│   └── src/ch01-ranks.md       # THE source: bower directives + play cells
├── bower_ipynb.py              # stand-in for `bower publish --target ipynb`
├── notebooks/ch01-ranks.ipynb  # generated + executed output (do not edit)
├── Dockerfile                  # FROM folkengine/spark4 + pkcore.py
├── docker-compose.yml
└── Makefile                    # make build / verify / up / local
```

## What it proves

| Spec claim | Status |
|---|---|
| § 15.1 play cell = new block kind, `notebook="play"`, never in the repo tree | done |
| § 15.1 attaches to nearest preceding step, or explicit `step=` | done |
| § 15.1 three plan-time errors (no step / unknown step / tree keys on a play cell) | done, exercised |
| § 3.4 display markers + `#` hidden lines decide what the reader sees | done |
| § 3.4 line-anchored links computed from the folded tree, drift is a build error | done — watch `src/rank.rs` L16–26 become L16–28 when the region grows |
| § 15.3 pinned wheel as the first cell | done (pinned to `pkcore.py==0.11.0` for the POC) |
| § 15.3 `verify --target ipynb`: execute headless, an erroring play cell fails the build | done |
| Open question 8 — `expect="raises"` on a play cell, controlled failing across the language boundary | done: the `Card.parse("Zz")` cell must raise or verify fails |

Verified output on the real wheel:

```
wrote notebooks/ch01-ranks.ipynb: 3 steps, 4 play cells, 17 cells
  cell  5  step=rank-enum              expect=pass    raised=False ok
  cell  9  step=rank-from-char-fail    expect=raises  raised=True  ok
  cell 14  step=rank-from-char         expect=pass    raised=False ok
  cell 16  step=rank-from-char         expect=pass    raised=False ok
verify: ok
```

## What it fakes

- **The wheel is `pkcore.py` from PyPI**, not a `bindings/` crate grown by the
  book and built at the `ch01-end` tag (§ 15.2). That is the real remaining
  work, and it is a Bower Phase 2/3 concern (replay + verify), not a notebook
  concern — the notebook side does not care where the wheel comes from.
- **The Rust blocks are sketches** of the § 9 rank saga, not pkcore's source.
  Correct shape, illustrative content.
- **Python, not `bower-core`.** ~250 lines standing in for the kernel's
  parse → group → fold → render path so the target could be tried today.
  Everything here is a port, not a design change: the pure parts (`parse`,
  `fold`, `shown_lines`, `line_range`) map 1:1 onto `bower-core`; only the
  `.ipynb` writer and `nbclient` runner are I/O.
- No `after=` ordering, no `delete`/`copy` ops, single chapter.

## Run it

```sh
make local                # no Docker: pip install pkcore.py nbformat nbclient ipykernel
docker compose build && make build verify && make up   # spark4 route, then http://localhost:8888
```

## Notes on the foundation

`folkengine/spark4` already ships JupyterLab, rustup, `evcxr_jupyter`, maturin
and PyO3 — everything § 15 needs, plus the evcxr Rust kernel that § 15.4 set
aside as a default but keeps as a per-book mode. `ImperialBower/pknotebook`
("adds pkcore and plpy to the Jupyter All Spark Notebook") is effectively
this Dockerfile already; the POC image could just be that image plus
`bower-ipynb` on the path. PySpark is dead weight for a book reader — a
slimmer `jupyter/minimal-notebook` base with the same Rust layer would cut
the image dramatically for the reader-facing edition.
