# The First Perspective

## TL;DR

Taking our first look at our repository. Get the code to run locally, isolate the problems, and
fix them.

------- 

Software systems are all about perspectives. When I am exploring a problem, I like to start with
mapping out the different perspectives that will be interacting with it. For example, let's take
a game of chess. In this case, an blindfold chess exhibition match between the #1 player in the world,
[Magnus Carlsen](https://en.chessbase.com/post/ice-barcelona-2026-exhibition), and the 
#2 [Hikaru Nakamura](https://hikarunakamura.com/). 

Take a minute, and try to picture all of there different perspectives that can exist on a simple, 
15 minute game.

Here's what I came up with:

- Magnus while playing
- Hikaru while playing
- Each of the players coaching.
- Each of the players past experiences playing each other.
- The live, [play by play on Take Take Take](https://www.youtube.com/live/_gL44mmsLlI?t=5911s) by GM David Howell and WFM Maud Rødsmoen.
- GothamChess aka Levy Rozman and GM Benjamin Bok doing play by play on [Hikaru's YouTube Channel](https://www.youtube.com/watch?v=UlwCgM5x9IM).
- GothamChess aka Levy Rozman giving his famous brand of [analysis of the game](https://www.youtube.com/watch?v=1YXsf6cmY4Y) on his YouTube channel.
- A [playable web version of the game](https://www.chess.com/events/ice-2026-fira-barcelona-event/01/Carlsen_Magnus-Nakamura_Hikaru) with analysis by Stockfish on Chess.com.
- [Analysis of the game](https://en.chessbase.com/post/ice-barcelona-2026-exhibition) on the chess database Chessbase.
- Some enlightening [reddit commentary](https://www.reddit.com/r/chess/comments/1qhz6i6/woah_didnt_even_know_ow_this_match_was_happening/).- 

Every system has an innate set of perspectives in how people interact with it, and there is always
going to be one that is foundational. For instance, in the above chess game, it might be argued that it's
this:

```txt
[Event "ICE Barcelona Blindfold Exhibition"]
[Site "Fira Barcelona, ESP"]
[Date "2026.01.20"]
[White "Carlsen, Magnus"]
[Black "Nakamura, Hikaru"]
[Result "1/2-1/2"]

1. Nf3 d5 2. e3 Nf6 3. c4 e6 4. Nc3 b6 5. b3 Bb7 6. Bb2 Bd6 
7. cxd5 exd5 8. Rc1 a6 9. Ne2 O-O 10. g3 Nbd7 11. Bg2 Re8 
12. O-O Bf8 13. Qc2 c5 14. d4 a5 15. Rfd1 Rc8 16. dxc5 Nxc5
17. Nc3 Qe7 18. Nd4 Nfe4 19. Qe2 g6 20. Ndb5 Rcd8 21. Na4 Bh6
22. Bd4 Ba6 23. Bxc5 Nxc5 24. Nxc5 bxc5 25. Bxd5 Bxe3 
26. fxe3 Rxd5 27. Rxd5 Qxe3+ 28. Qxe3 Rxe3 29. Rdd1 Bxb5
30. Rxc5 Be2 31. Re1 Re6 32. Rxa5 Kf8 33. Kf2 Bg4
34. Rxe6 Bxe6 35. Ke3 Ke7 36. Kd4 Kd6 37. Ra6+ Kc7 1/2-1/2
```

A simple, text representation of the game. But that's the final outcome, not the foundation. Could
it be said that it's the very game itself? The pieces, on the board, governed by rules, passed
down over generations. 

For software developent, I maintain that the foundation is the software developer, and unfortunately, 
probably the one that is least appreciated in modern enterprise. 
Your perspective on creating systems is what matters most. Our goal is to make things as smooth
as possible for you, or any other developer to work on something. In the case of this book, I've
designed it so that you can walk in at any point, and start working on the next, at any time. I've
tried to make it as easy possible for you to code along, making this an experience were you are 
an essential part of the process. 

As we go along, building our poker engine, we will be adding more and more perspectives, as our system
matures. Right now there is only one: you... the developer. 

It's my not so humber opinion, that how little companies consider this, is one of the most needless
wastes of money in our industry. Case in point: 

> _Story time. Y'all can skip these things. I wouldn't be a Baker if I didn't litter this book 
> with stories._ 
> 
> It was my first day working on an engagement for a major clothing retailer.
> 
> _"So, what are you working on?"_
> 
> The question came from the gentleman sitting next to me on the open floor of their stunning 
> corporate main office. _"Nothing. I don't access yet."_
> 
> He laughed. _"Oh, your signing bonus."_
> 
> _"My what?"_
> 
> _"Your signing bonus. Getting paid to do nothing... that's your signing bonus. Here it's around two weeks."_

Over the years, I have see signing bonuses that have gone on for months. I would bet that there are many
who's signing bonus lasts for years. 


I hate initial setup chapters in coding books. Instead, let's start our project's codebase
with a really messed up version based on my [World's Simplest Kata](https://github.com/devplaybooks/rust_worlds_simplest_kata). It's designed
to show you many of the different ways that Rust code can fail continuous integration tools
like GitHub Actions. 

Here are the files for our initial commit:

```shell
.github/workflows/CI.yaml
.gitignore
Cargo.lock
Cargo.toml
LICENSE-APACHE
LICENSE-MIT
src/lib.rs
```

## Cargo.toml

We start with `Cargo.toml`, the manifest file that defines the key elements of our Rust project. You can see a breakdown
of the entries in [The Cargo Book](https://doc.rust-lang.org/cargo/reference/manifest.html).

<!-- bower repo="rust4failures" step="kata" file="Cargo.toml" expect="compile_fail" msg="init" -->

```toml
[package]
name = "rust4failures"
description = "Poker library."
version = "0.0.1"
edition = "2024"
rust-version = "1.98.1"
authors = ["electronicpanopticon <gaoler@electronicpanopticon.com>", "Readers"]
license = "MIT OR Apache-2.0"
keywords = ["rust", "book", "poker", "wasm"]

[dependencies]

[dev-dependencies]

[lints.clippy]
pedantic = "warn"
```

Up next is `Cargo.lock`. It's autogenerated when you run `cargo build`. Don't
touch it. We are committing it. 

<!-- bower repo="rust4failures" step="kata" file="Cargo.lock" msg="init" -->

```toml
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "rust4failures"
version = "0.0.1"
```

### Linting

One thing that I want to call out is this line `pedantic = "warn"` under `lints.clippy`. I love linters
on my projects, and add them whenever I can. It's best to use them from the start, since the longer
you wait, the more you have to clean up when you add it. 

Rust's [Clippy linter](https://doc.rust-lang.org/clippy/index.html) is the best in the business. And
since I'm insane, I like to [dial it up to 11](https://en.wikipedia.org/wiki/Up_to_eleven) with the 
[pedantic setting](https://doc.rust-lang.org/clippy/lints.html#pedantic). 
![Spinal_Tap_-_Up_to_Eleven.jpg](images/Spinal_Tap_-_Up_to_Eleven.jpg)

> 💡LESSON: Linters are a powerful way to keep your code clean, and catch basic coding issues. 

## GitHub Actions

This is our traffic cop. There are many different ways to setup continuous integration, but
[GitHub Actions](https://docs.github.com/en/actions) are one of my favorites. I'd recommend
that you check out the entire file for our CI guardrails. For now, I've highlighted two sections:
`on` and the `test` job. On tells us that we are checking everything every time there's a push
or a pull request. After a while, that may be too much, and we can dial it down, but for now
it's good. 

<!-- bower repo="rust4failures" step="kata" file=".github/workflows/CI.yaml" msg="init" -->

```yaml
name: CI

# bower:show
on:
  push:
  pull_request:
# bower:show end

permissions:
  contents: read

env:
  RUSTFLAGS: -Dwarnings

# bower:show
jobs:
  test:
    name: Rust ${{matrix.rust}}
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        rust: [stable, 1.98.1]
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{matrix.rust}}
      - run: cargo test --all
        # bower:show end

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    if: github.event_name != 'pull_request'
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: clippy
      - run: cargo clippy -- -Dclippy::all -Dclippy::pedantic

  fmt:
    name: Fmt
    runs-on: ubuntu-latest
    if: github.event_name != 'pull_request'
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: rustfmt
      - run: cargo fmt --all -- --check

  doc:
    name: Doc
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: rust-docs
      - run: cargo doc --no-deps --document-private-items
        env:
          RUSTDOCFLAGS: "-D warnings"

```

## lib.rs

Now down to business. This is a simple `Hello, World` example. It's all going to be gone in a 
bit, but it's a good way to kick the tires on how we are ensuring quality in this project.

<!-- bower repo="rust4failures" step="kata" file="src/lib.rs" msg="init" -->

```rust
pub struct Hello;

/// The [Helo] project is a simple hello world program. It comes with two methods:
///
/// 1. `hello` - This method takes a `String` and returns `Hello, {name}!`.
/// 2. `hello_world` - A default method that returns `Hello, world!`.
///
/// ```
/// use worlds_simplest_kata::Hello;
///
/// let hello = Hello::hello("everyone".to_string());
/// assert_eq!("Hello, everyone.", hello);
///
/// ```
impl Hello {
    pub fn hello(name: String) -> String {
        format!("Hello,  {}!", name)
    }

    pub fn  hello__world() -> &'static str {
        Hello::hello("wirld!".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world() {
        assert_eq!("Hello, world!", Hello::hello__world());
    }
}
```

```
            ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
            ()-()                            ()-()
             \"/     NOW IT'S YOUR TURN!      \"/
              `                                ` 
```




````markdown
`hello` hands back an owned `String`. `hello__world` promises a borrowed
`&'static str`. Both cannot be true, so one of the two has to move.

- The compiler suggests `&Hello::hello(…)`. Take the suggestion, run
  `cargo check` again, and read what it says next.
- The other direction changes the signature. Which one leaves the caller —
  the test at the bottom of the file — still working?
- Everything else in this file is still wrong: the doc test names a crate that
  does not exist, the greeting has two spaces, the world is spelled `wirld`,
  and the method has two underscores. Leave them. Compiling and being correct
  are different jobs, and this one is compiling.
````

## The one-line answer

The signature moves, not the body. `hello__world` returns exactly what `hello`
returns, and now it says so.

<!-- bower repo="rust4failures" step="kata-compiles" file="src/lib.rs" op="replace" expect="test_fail" msg="fix: hello__world returns what hello returns" -->

```rust
pub struct Hello;

/// The [Helo] project is a simple hello world program. It comes with two methods:
///
/// 1. `hello` - This method takes a `String` and returns `Hello, {name}!`.
/// 2. `hello_world` - A default method that returns `Hello, world!`.
///
/// ```
/// use worlds_simplest_kata::Hello;
///
/// let hello = Hello::hello("everyone".to_string());
/// assert_eq!("Hello, everyone.", hello);
///
/// ```
impl Hello {
    pub fn hello(name: String) -> String {
        format!("Hello,  {}!", name)
    }

    // bower:show
    pub fn  hello__world() -> String {
        Hello::hello("wirld!".to_string())
    }
    // bower:show end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world() {
        assert_eq!("Hello, world!", Hello::hello__world());
    }
}
```

One word. The block holds the whole file, because the repository needs the
whole file, but only the method was printed on the page.

`cargo check` is green. `cargo test` is not, which is why this step declares
`expect="test_fail"`:

```console
$ cargo test
thread 'tests::hello_world' panicked at src/lib.rs:31:9:
assertion `left == right` failed
  left: "Hello, world!"
 right: "Hello,  wirld!!"
```

> 💡LESSON: "It compiles" is the first gate, not the last one. A green compiler
> only means the code is well formed. The tests are what say it is right.
