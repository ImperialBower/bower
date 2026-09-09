# The command surface

One global flag, `--book <DIR>` (default `.`), names the book root holding
`bower.toml`. Config is loaded before dispatch, so a bad `bower.toml` fails every
subcommand identically. An unmatched `--repo` prints `bower: no repo matched`
and exits non-zero in every command — empty output plus exit 0 reads as success.

# The loop

* [bower plan](plan.md) - resolve the book, print the plan, write `bower.lock`.
* [bower build](build.md) - replay the plan into a git repository, one commit per step.
* [bower verify](verify.md) - run each step's declared expectation against a real compiler.
* [bower status](status.md) - report drift between book, lock, built repo, and rendered site.

# Shipping

* [bower publish](publish.md) - render the book: `--target html | epub | pdf`.
* [bower push](push.md) - publish the repository, its site, and its release. Dry run unless `--execute`.

# Not yet built

`--target ipynb` is specified but unimplemented — see
[the notebook target](/roadmap/notebook-target.md).
