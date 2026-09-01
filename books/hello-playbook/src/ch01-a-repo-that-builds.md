# A repo that builds

Every project starts the same way: a manifest that names it, and one file that
runs. Nothing else has earned its place yet.

<!-- bower repo="hello-playbook" step="cargo-init" file="Cargo.toml" msg="feat: a package that builds" -->

```toml
[package]
name = "hello-playbook"
version = "0.1.0"
edition = "2021"
rust-version = "1.95"
license = "MIT OR Apache-2.0 OR GPL-3.0-or-later"

[dependencies]

# bower:begin lints
# bower:end lints
```

The two comment lines at the bottom are a *region marker*. They mark a hole in
the file that a later chapter will fill, and they are stripped out before the
file reaches the repository. Chapter 3 fills this one.

<!-- bower repo="hello-playbook" step="cargo-init" file="src/main.rs" -->

```rust
fn main() {
    println!("Hello, world!");
}
```

Both blocks above carry `step="cargo-init"`, so they land in one commit. That is
right: a manifest without a source file is not a state of the project anyone
would want to check out.

## Running it

<!-- bower repo="hello-playbook" step="hello-runs" op="none" msg="docs: cargo run prints the greeting" -->

This step writes no files. `op="none"` declares a commit that exists only to
mark a moment in the narrative — here, the moment you can first run the thing.
The command is:

```console
$ cargo run
Hello, world!
```

That fence has no directive above it, so Bower ignores it. Only fenced blocks
introduced by a `<!-- bower … -->` comment become code in the repository.
