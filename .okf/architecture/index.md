# Architecture

How Bower is put together, and why the seams are where they are.

# Shape

* [The pipeline](pipeline.md) - book markdown to blocks, steps, plan and tree states — then git, the compiler, and the renderers.
* [The domain kernel pattern](domain-kernel.md) - a pure core with the purity asserted in CI, not assumed.
* [The invariants](invariants.md) - what the system promises, and the suite that proves each promise.

# The crates

* [bower-core](crate-bower-core.md) - the domain kernel. Zero dependencies.
* [bower (the CLI)](crate-bower.md) - the I/O half: git, subprocesses, config, network.
* [bower-testkit](crate-bower-testkit.md) - fixtures, generators, and a state-coverage report.
* [mdbook-bower — the preprocessor](mdbook-bower.md) - a second binary in the CLI crate that rewrites chapters at book-build time.
