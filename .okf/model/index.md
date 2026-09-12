# The domain model

The vocabulary Bower thinks in. These are the pure concepts the
[kernel](/architecture/domain-kernel.md) computes over — no git, no filesystem,
no renderers.

# Input

* [Book source](book.md) - chapters in reading order, a block library, and assets — everything the kernel is allowed to see.
* [The bower directive](directive.md) - the HTML comment that binds a fenced block to a file in a target repository.

# Resolution

* [Step](step.md) - one commit-to-be in one repo, grouping one or more blocks.
* [Ordering](ordering.md) - document order by default, bent by `after=`, cycles refused.
* [Plan](plan.md) - the fully-resolved, ordered list of steps — a pure value.
* [Tree state](tree-state.md) - the complete file tree after a step, deterministic by construction.

# What the reader sees

* [Assembly — fragments vs. full files](assembly.md) - hidden lines and named regions, the two ways one block is both fragment and whole file.
* [Display markers and elision](display-markers.md) - which spans render, and the one rule where HTML and epub/PDF differ.
* [Exercise](exercise.md) - a step as a place to stop reading and start typing.
* [Captured output](captured-output.md) - what the compiler said at a step, recorded in the book and checked on every verify.
* [Play cell](play-cell.md) - a live Python cell bound to a step, for the notebook target.

# When it goes wrong

* [Expectation — controlled failure as an executable claim](expectation.md) - `pass`, `compile_fail`, `test_fail`, `none`.
* [Errors — located and collected](errors.md) - 37 variants, each carrying its chapter and line.
