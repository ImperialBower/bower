# Tests, and failing on purpose

A test that has never failed has never been tested. This chapter writes one that
fails, commits it in that state, and fixes it in the next commit — so the
failure is a real, tagged, checkoutable state of the repository rather than
something that happened off-camera.

## A library to test

<!-- bower repo="hello-playbook" step="greet-lib" file="src/lib.rs" msg="feat: greet(), extracted into a library" -->

```rust
//! A greeting, and nothing else.

// bower:begin mods
// bower:end mods
// bower:begin greet
/// Build a greeting for `name`.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
// bower:end greet
```

The `mods` region is empty and has no blank line around it. Its blank line will
arrive as part of its contents later, so that emptying the region again does not
leave a double blank line behind — which the formatter, installed in chapter 3,
would reject.

<!-- bower repo="hello-playbook" step="greet-lib" file="src/main.rs" op="replace" -->

```rust
use hello_playbook::greet;

fn main() {
    println!("{}", greet("world"));
}
```

`op="replace"` writes a whole new file over the old one. Both blocks share
`step="greet-lib"`, so extracting the function and rewiring the binary are one
commit — the repository never passes through a state where `main.rs` refers to
something that is not there yet.

## A test that passes

<!-- bower repo="hello-playbook" step="greet-test" file="src/lib.rs" op="append" msg="test: greet uses the name it is given" -->

```rust

// bower:begin tests
#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet_uses_the_name() {
        assert_eq!(greet("world"), "Hello, world!");
    }
}
// bower:end tests
```

`op="append"` adds to the bottom of a file without repeating its top. The block
opens with a blank line, which is how the separating blank line gets there.

## A test that fails

<!-- bower repo="hello-playbook" step="test-that-fails" file="src/lib.rs" op="region" region="tests" expect="test_fail" msg="test: greet should ignore stray whitespace (failing)" exercise="Make the test pass" -->

```rust
#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet_uses_the_name() {
        assert_eq!(greet("world"), "Hello, world!");
    }

    // bower:show
    #[test]
    fn greet_ignores_stray_whitespace() {
        assert_eq!(greet("  world  "), "Hello, world!");
    }
    // bower:show end
}
```

Two things are happening here.

`expect="test_fail"` declares that this commit's tests are *supposed* to fail.
That is a promise the toolchain checks: when the replay layer builds this
repository it runs the tests at this step and treats a pass as the error.

The `bower:show` markers decide what you just read. The block above contains the
whole test module, because the repository needs the whole file — but only the
new test was printed on the page. The markers never reach the repository.

The `exercise` key is the third thing. It marks this step as a place to stop
reading and start typing: the box under the code above tells you how to fork
the repository, check out exactly this commit, and run the tests yourself. The
next section is one answer, and it stays folded until you open it.

Here is the part of what `cargo test` prints at this step that matters:

<!-- bower repo="hello-playbook" output="verify" -->

```text
[...]
thread 'tests::greet_ignores_stray_whitespace' panicked at src/lib.rs:20:9:
assertion `left == right` failed
  left: "Hello,   world  !"
 right: "Hello, world!"
[...]
```

That is not a paste. The `output="verify"` directive above binds the block to
this step, and `bower verify` runs the tests and fails the day what they print
changes. `bower verify --record` is what filled it in, with the whole output.

The two `[...]` lines are a trim, made by hand after recording. Each one stands
for lines left out, so the quote can skip the test list and the summary and
keep only the failure. It is still checked: the lines that remain must appear
in the output, in this order. `--record` leaves the trim alone for as long as
that holds, and writes the whole output back the day it does not.

## The fix

<!-- bower repo="hello-playbook" step="test-that-passes" file="src/lib.rs" op="region" region="greet" msg="fix: greet ignores stray whitespace" -->

```rust
/// Build a greeting for `name`, ignoring stray whitespace.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name.trim())
}
```

The fix is in a different region of the same file. That is the point of regions:
the test module and the function it tests grow independently, and each edit
shows only itself.

## Code that does not compile

<!-- bower repo="hello-playbook" step="wont-compile" file="src/scratch.rs" expect="compile_fail" msg="feat: scratch module (does not compile)" -->

```rust
// bower:begin scratch
/// Deliberately wrong: a string literal is not a `u32`.
#[must_use]
pub fn answer() -> u32 {
    let n: u32 = "42";
    n
}
// bower:end scratch
```

<!-- bower repo="hello-playbook" step="wont-compile" file="src/lib.rs" op="region" region="mods" -->

```rust
pub mod scratch;

```

The second block is not decoration. A file that no module declaration points at
is never compiled, so a `compile_fail` in an undeclared file would be a lie
about the toolchain. Declaring the module and writing the broken code are one
commit, and that commit genuinely does not build.

Notice the blank line at the end of that block. That is the separator promised
earlier — it belongs to the region, not to the file around it.

And here is what the compiler says about it, checked the same way:

<!-- bower repo="hello-playbook" output="check" -->

```text
error[E0308]: mismatched types
 --> src/scratch.rs:4:18
  |
4 |     let n: u32 = "42";
  |            ---   ^^^^ expected `u32`, found `&str`
  |            |
  |            expected due to this

For more information about this error, try `rustc --explain E0308`.
error: could not compile `hello-playbook` (lib) due to 1 previous error
```

## The repair

<!-- bower repo="hello-playbook" step="scratch-fixed" file="src/scratch.rs" op="region" region="scratch" msg="fix: parse the text instead of pretending it is a number" -->

```rust
/// Parse the text, and fall back to zero when it is not a number.
#[must_use]
pub fn answer() -> u32 {
    "42".parse().unwrap_or_default()
}
```

`unwrap_or_default` rather than `unwrap`: chapter 3 set `unwrap_used = "warn"`,
and the gate turns warnings into failures.
