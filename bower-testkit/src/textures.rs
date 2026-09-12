//! Raw command output, as a machine printed it: the data textures of
//! `bower_core::capture::normalize`. Each is real text, captured on 11
//! September 2026, with only the scratch directory replaced by [`SCRATCH`] —
//! except [`TIMED_PASS_SHUFFLED`], which says how it was made.

use bower_core::prelude::Scrub;

/// Where these textures' scratch tree and target directory lived.
pub const SCRATCH: &str = "/private/var/folders/zz/T/bower-verify/book";

/// The scrub `bower verify` would build for [`SCRATCH`].
#[must_use]
pub fn scrub() -> Scrub {
    Scrub(vec![
        (format!("{SCRATCH}/tree/"), String::new()),
        (format!("{SCRATCH}/tree"), ".".to_string()),
        (format!("{SCRATCH}/target"), "target".to_string()),
    ])
}

/// `cargo check` on a non-exhaustive `match` over `char`: a note, a help, and
/// spans — the shape `rust4failures` chapter 3 teaches.
pub const E0004: &str = r"    Checking rank v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
error[E0004]: non-exhaustive patterns: `'\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered
  --> src/lib.rs:8:15
   |
 8 |         match c {
   |               ^ patterns `'\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered
   |
   = note: the matched value is of type `char`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern as shown, or multiple match arms
   |
10 ~             'K' | 'k' => Rank::KING,
11 ~             _ => todo!(),
   |

For more information about this error, try `rustc --explain E0004`.
error: could not compile `rank` (lib) due to 1 previous error
";

/// [`E0004`] with `CARGO_TERM_COLOR=always`.
pub const E0004_ANSI: &str = "\x1b[1m\x1b[92m    Checking\x1b[0m rank v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)\n\x1b[1m\x1b[91merror[E0004]\x1b[0m\x1b[1m: non-exhaustive patterns: `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered\x1b[0m\n  \x1b[1m\x1b[94m--> \x1b[0msrc/lib.rs:8:15\n   \x1b[1m\x1b[94m|\x1b[0m\n\x1b[1m\x1b[94m 8\x1b[0m \x1b[1m\x1b[94m|\x1b[0m         match c {\n   \x1b[1m\x1b[94m|\x1b[0m               \x1b[1m\x1b[91m^\x1b[0m \x1b[1m\x1b[91mpatterns `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered\x1b[0m\n   \x1b[1m\x1b[94m|\x1b[0m\n   \x1b[1m\x1b[94m= \x1b[0m\x1b[1mnote\x1b[0m: the matched value is of type `char`\n\x1b[1m\x1b[96mhelp\x1b[0m: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern as shown, or multiple match arms\n   \x1b[1m\x1b[94m|\x1b[0m\n\x1b[1m\x1b[94m10\x1b[0m \x1b[92m~ \x1b[0m            'K' | 'k' => Rank::KING\x1b[92m,\x1b[0m\n\x1b[1m\x1b[94m11\x1b[0m \x1b[92m~             _ => todo!()\x1b[0m,\n   \x1b[1m\x1b[94m|\x1b[0m\n\n\x1b[1mFor more information about this error, try `rustc --explain E0004`.\x1b[0m\n\x1b[1m\x1b[91merror\x1b[0m: could not compile `rank` (lib) due to 1 previous error\n";

/// [`E0004`] as a Windows runner prints it.
#[must_use]
pub fn e0004_crlf() -> String {
    E0004.replace('\n', "\r\n")
}

/// `hello-playbook` step `wont-compile`: a string literal where a `u32` goes.
pub const E0308: &str = r#"    Checking hello-playbook v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
error[E0308]: mismatched types
 --> src/scratch.rs:4:18
  |
4 |     let n: u32 = "42";
  |            ---   ^^^^ expected `u32`, found `&str`
  |            |
  |            expected due to this

For more information about this error, try `rustc --explain E0308`.
error: could not compile `hello-playbook` (lib) due to 1 previous error
"#;

/// `hello-playbook` step `test-that-fails`, both streams through one pipe:
/// cargo's status lines, libtest's report with the panicking thread's ID and
/// the clock, then cargo's closing line.
pub const TEST_PANIC: &str = r#"   Compiling hello-playbook v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.64s
     Running unittests src/lib.rs (/private/var/folders/zz/T/bower-verify/book/target/debug/deps/hello_playbook-75e8099ccb6a0c05)

running 2 tests
test tests::greet_ignores_stray_whitespace ... FAILED
test tests::greet_uses_the_name ... ok

failures:

---- tests::greet_ignores_stray_whitespace stdout ----

thread 'tests::greet_ignores_stray_whitespace' (714955) panicked at src/lib.rs:20:9:
assertion `left == right` failed
  left: "Hello,   world  !"
 right: "Hello, world!"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    tests::greet_ignores_stray_whitespace

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--lib`
"#;

/// `rust4failures` step `kata-compiles`: `cargo check` succeeds with one
/// warning.
pub const WARNING_ONLY: &str = r"    Checking rust4failures v0.0.1 (/private/var/folders/zz/T/bower-verify/book/tree)
warning: method `hello__world` should have a snake case name
  --> src/lib.rs:20:13
   |
20 |     pub fn  hello__world() -> String {
   |             ^^^^^^^^^^^^ help: convert the identifier to snake case: `hello_world`
   |
   = note: `#[warn(non_snake_case)]` (part of `#[warn(nonstandard_style)]`) on by default

warning: `rust4failures` (lib) generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.31s
";

/// A manifest with no targets: cargo names the scratch path itself — the
/// shape of `rust4failures` step `kata`, a lone `Cargo.toml`.
pub const MANIFEST_ERROR: &str = r"error: failed to parse manifest at `/private/var/folders/zz/T/bower-verify/book/tree/Cargo.toml`

Caused by:
  no targets specified in the manifest
  either src/lib.rs, src/main.rs, a [lib] section, or [[bin]] section must be present
";

/// Three passing tests and the doc-test run, timed.
pub const TIMED_PASS: &str = r"   Compiling three v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.92s
     Running unittests src/lib.rs (/private/var/folders/zz/T/bower-verify/book/target/debug/deps/three-b48f404885b51d48)

running 3 tests
test tests::alpha ... ok
test tests::beta ... ok
test tests::gamma ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests three

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

";

/// [`TIMED_PASS`] edited by hand into what another multi-threaded run prints:
/// another order, another clock, another hash. The one texture not captured
/// as-is — one thread per step (EPIC-11 Decision 12) means `verify` never
/// sees this, but a reader's own run does.
pub const TIMED_PASS_SHUFFLED: &str = r"   Compiling three v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.07s
     Running unittests src/lib.rs (/private/var/folders/zz/T/bower-verify/book/target/debug/deps/three-0c1d2e3f4a5b6c7d)

running 3 tests
test tests::gamma ... ok
test tests::alpha ... ok
test tests::beta ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests three

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

";
