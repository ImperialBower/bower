# Hell in a Cell

## TL;DR

I hate initial setup chapters in coding books. Just do the 
[The World's Simplest Kata](https://github.com/devplaybooks/rust_worlds_simplest_kata) to make sure you have everything setup correctly.

------

# EDITOR: This all may suck. Could just leave the top part. 

------

ANNOUNCER:
> *Two programmers walk in, one walks out. Who will triumph...*

`     `💥💥EXPLOSIONS💥💥</br>
`     `QUEUE [Gonna Fly Now aka Rocky Theme](https://www.youtube.com/watch?v=_YYmfM2TfUA)

> *the naive young developer trying to thrive in our cruel technocratic society...*

`     `💥💥LASER SHOW💥💥</br>
`     `QUEUE [Flight of the Concords' Robots](https://youtu.be/2IPAOxrH7Ro?t=86)
>
> *or Broboto, the Soulless Savage AI Robot, trained by Silicon Valley, hellbent on terminating anyone who dares to clutch a keyboard?*

Sure, every sociopathic CEO on the planet is trying to replace you with AI, but can they? These programming streets are hard, and the defects don't play by [Marquess of Queensberry Rules](https://en.wikipedia.org/wiki/Marquess_of_Queensberry_Rules). Right now hundreds of thousands of clueless users and vapid script kiddies are foaming at the mouth to do incredibly stupid and cringeworthy things to any program released into the wild.

Are you going to run back to mommy's safe warehouse job or are you going to step to it and take on the AI horde? Do you have what it takes? Do you have the intestinal fortitude to enter the

```txt
██╗  ██╗███████╗██╗     ██╗         ██╗███╗   ██╗     █████╗ 
██║  ██║██╔════╝██║     ██║         ██║████╗  ██║    ██╔══██╗
███████║█████╗  ██║     ██║         ██║██╔██╗ ██║    ███████║
██╔══██║██╔══╝  ██║     ██║         ██║██║╚██╗██║    ██╔══██║
██║  ██║███████╗███████╗███████╗    ██║██║ ╚████║    ██║  ██║
╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝    ╚═╝╚═╝  ╚═══╝    ╚═╝  ╚═╝
                                                             
 ██████╗███████╗██╗     ██╗   ██████╗                        
██╔════╝██╔════╝██║     ██║   ╚════██╗                       
██║     █████╗  ██║     ██║     ▄███╔╝                       
██║     ██╔══╝  ██║     ██║     ▀▀══╝                        
╚██████╗███████╗███████╗███████╗██╗                          
 ╚═════╝╚══════╝╚══════╝╚══════╝╚═╝                             
```
## The Cell

You and your AI opponent will be locked into a Cell of the [Rust](https://www.rust-lang.org/) programming language. Why Rust? Because it's difficult, and cruel, and the programmers who code in it are all assholes, and naturally, since I am an asshole, it is my favorite language. *Seriously, because of it's strict nature it's the perfect antidote to the AI vibe sludge coming out these days.*

The problem is simple: Hello, World!. Yes, [Hello, motherfucking World!](https://en.wikipedia.org/wiki/%22Hello,_World!%22_program). The most overused programming exercise on the planet; Beaten to death so hard that it's the turned into the finest McRib worthy [pink slime](https://en.wikipedia.org/wiki/Pink_slime).
### The Opposition

Your opponent is CoPilot. Yes, CoPilot; Sam Altman's & Satya Nadella's purloined AI love child currently infesting every Microsoft based application it can sink it's binary claws into. *Oh my, how did I ever use Notepad before CoPilot? Thank you! Thank you!!! I'll never had to worry about how to end a sentence again!*

GitHub has added an adorable new feature: you can now get code reviews from CoPilot. So, I've taken the error littered original version of the [kata](https://github.com/devplaybooks/rust_worlds_simplest_kata), and ran it through a CoPilot code review, to see how well it can do.

## Let us go!

Here's your mission:

- Fork the  [The World's Simplest Kata](https://github.com/devplaybooks/rust_worlds_simplest_kata) repo.
-
- Make the [failing GitHub Actions](https://github.com/devplaybooks/rust_worlds_simplest_kata/actions) green.

-----

Here's were you should stop and try to work through it yourself. If you're still having issues
here's a walkthrough of me getting things to work:

Step 1: Install Rust

➔ TL;DR GOTO [rustup.rs](https://rustup.rs/). Do what it says.

Rust has the best ecosystem for a programming language in the world. If you've banged your head against the wall trying to get a grip on how to manage Python projects, or lost yourself in a sea of gcc compiler flags, Rust will seem like a cool ocean's breeze.

- Install [rustup](https://rustup.rs/).
- run `rustup install`

You should see something like this:

```shell
$> rustc --version
rustc 1.97.1 (8bab26f4f 2026-07-14
```

## Step 2: Fork it

IF you have a github account:
fork the repo
enable GitHub Actions for your fork.[^1]
ELSE if *brah, I don't mess with those giganto corporations, man*:
download the code onto your machine.

## Question: Does it build?

```rust
$> cargo build
`/Users/flubble/src/github.com/devplaybooks/rust_worlds_simplest_kata/Cargo.toml`

Caused by:
  rust-version 1.72.1 is incompatible with the version (1.85.0) required by the specified edition (2024)
```

It's insane how good the Rust compiler is at spelling out exactly what the problem is. The `../../../Cargo.toml` file says `edition = "2024"` but the Rist versions is set to `1.72.1`. Let's bump it up to what we're running, version `1.97.1`:

```diff
diff --git a/Cargo.toml b/Cargo.toml
index 7220334..26bb72e 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -3,7 +3,7 @@ name = "worlds_simplest_kata"
 version = "0.1.2"
 authors = ["electronicpanopticon <gaoler@electronicpanopticon.com>"]
 edition = "2024"
-rust-version = "1.72.1"
+rust-version = "1.97.1"
 description = "A kata to learn how to fix basic issues with rust code."
 repository = "https://github.com/devplaybooks/rust_worlds_simplest_kata"
 license = "GPL-3"
```
### Step 4: Does it build now?

Let's see how we're doing:

```rust
$> cargo build 
   Compiling worlds_simplest_kata v0.1.2 (/Users/christoph/src/github.com/abstecker/worlds_simplest_kata_no_actions)
error[E0308]: mismatched types
  --> src/lib.rs:23:9
   |
22 |     pub fn  hello__world() -> &'static str {
   |                               ------------ expected `&'static str` because of return type
23 |         Hello::hello("wirld!".to_string())
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `&str`, found `String`
   |
help: consider borrowing here
   |
23 |         &Hello::hello("wirld!".to_string())
   |         +

For more information about this error, try `rustc --explain E0308`.
error: could not compile `worlds_simplest_kata` (lib) due to 1 previous error
```

Rust strings still terrify me. I feel like I am a lot better at dealing with them then when I first started out, but still... damn do they call out all of my deficiencies as a developer. My php, perl coding early ass didn't know shit... didn't learn shit... was just making it up as I went along... *ASIDE: I still am.*

Basically, a [String](https://doc.rust-lang.org/std/string/struct.String.html) is is stored in the heap[^2] of your program. It's like that extra suitcase you take on a trip so that you can fit in silly nick knacks you collect along your way. A [str primitive type](https://doc.rust-lang.org/std/primitive.str.html), on the other hand, is something that you can borrow, but not buy. It's like that key to the gas station restroom, chained to an old license plate. You can borrow it, but it's not yours.

In this case, the `hello__world` function is returning `&'static str`; what's called a string literal. It's a string that's burned into your program, and will exist unti it ends. It's like that tattoo you got that one night, in that place were no one can see it... you know the one.

-----

I can see three paths forward. Here's the simplest. I have a static function that's trying to return a static str, that insteads returning the results of a function that returns a String.

> Patient: *Doc, it hurts when I hold my arm like that.*
> Doctor: *Well don't hold arm like that.*

So, let's do what it says:

```rust
pub fn  hello__world() -> &'static str {  
    "Hello, world!"  
}
```

Now we're returning a pure String literal. Let's see how it does.

```shell
$> cargo build                                                                                                                                             ─╯
warning: method `hello__world` should have a snake case name
  --> src/lib.rs:22:13
   |
22 |     pub fn  hello__world() -> &'static str {
   |             ^^^^^^^^^^^^ help: convert the identifier to snake case: `hello_world`
   |
   = note: `#[warn(non_snake_case)]` (part of `#[warn(nonstandard_style)]`) on by default

warning: `worlds_simplest_kata` (lib) generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
```

It builds, but with a warning. First, ret's get rid of that warning, and remove the dunder, or double underscore, from the `hello__world()` function.

> Did you know that a double underscore in some programmering circles is called a dunder? I learned that while attending one of the [Central Ohio Python User's Group](http://cohpy.org/) meetings. In python code you will see functions named [____init____ ](https://www.geeksforgeeks.org/python/__init__-in-python/), and they would call them "dunder inits."

Here's the update to the function that matches the recomentation from error message as close as possible:

```rust
pub fn hello_world() -> &'static str {  
   "Hello, world!"  
}
```
Let's see how that does.
```bash
$> cargo build
   Compiling worlds_simplest_kata v0.1.2 (/Users/christoph/src/github.com/devplaybooks/rust_worlds_simplest_kata)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.00s
```
Nice... it builds fine.
```shell
cargo run
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running `target/debug/worlds_simplest_kata`
Hello, world!
```
And it runs without a problem.

Let's see if the tests are green:

```bash
$> cargo test
Compiling worlds_simplest_kata v0.1.2 (/Users/christoph/src/github.com/devplaybooks/rust_worlds_simplest_kata)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.15s
     Running unittests src/lib.rs (target/debug/deps/worlds_simplest_kata-c6a62a2629fcb368)

running 1 test
test tests::hello_world ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/worlds_simplest_kata-cfecd8ab1322d86e)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
yes, but...

```bash
   Doc-tests worlds_simplest_kata
running 1 test
test src/lib.rs - Hello (line 10) ... FAILED
failures:
---- src/lib.rs - Hello (line 10) stdout ----
Test executable failed (exit status: 101).

stderr:

thread 'main' (876041) panicked at /var/folders/yc/zz4hyvrx0rzc5tdpkjgrkmy00000gn/T/rustdoctestCeSnEA/doctest_bundle_2024.rs:9:1:
assertion `left == right` failed
  left: "Hello, everyone."
 right: "Hello,  everyone!"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    src/lib.rs - Hello (line 10)

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

all doctests ran in 0.69s; merged doctests compilation took 0.36s
error: doctest failed, to rerun pass `--doc`
```
This is a failure in the test that is written inside the documentation for the code.

I love Rust's [documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html). One on the biggest challenges in keeping software up to date is the documentation. Changes will be made to the code, but everyone's in such a hurry, that they forget to keep the documentation up to date as well. Often, you will hear from developers that it's just better to just forget it, since it's often less than useless.

With doctests, not only do you ensure that you have up to date documentation, but you also provide examples to your users that you know are correct.

And, as is the case most of the time, the compiler spells out the problem. There's an extra space in the value returned from the `hello()` function.

Take 3:

```bash
$> cargo test
...
 left: "Hello, everyone."
right: "Hello, everyone!"
```
Almost... Let's fix the punctuation.

```bash
cargo test
   Compiling worlds_simplest_kata v0.1.2 (/Users/christoph/src/github.com/devplaybooks/rust_worlds_simplest_kata)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.49s
     Running unittests src/lib.rs (target/debug/deps/worlds_simplest_kata-c6a62a2629fcb368)

running 1 test
test tests::hello_world ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/worlds_simplest_kata-cfecd8ab1322d86e)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests worlds_simplest_kata

running 1 test
test src/lib.rs - Hello (line 10) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 0.61s; merged doctests compilation took 0.29s
```
Boom! We are green. Ship it!
```bash
$> git add .
$> git commit -m "Fixed failing tests, and compilation errors"
[bookwalker_fix1_static 7be5b8a] Fixed failing tests, and compilation errors
 1 file changed, 2 insertions(+), 2 deletions(-)
 $> git push
 ...
```

Let's see


```bash

```

There are, however, other ways...

...

...

...

TAKE 2:




Looking at the error, it's clear that they are mixing up the trwo

If you want to see a a microcosm that contains almost everything maddening about Rust, this file is it. Strings in Rust are pure pain.

Nope. We've still got a problem, and this one is the
and it points to what is probably the hardest thing to understand about Rust programming: strings. Rust pulls no punches. If you come from a scripting background like JavaScript or Python, Rust strings are going to seem like pain incarnate. Unlike most other languages, Rust puts it all out there. Strings aren't easy.

Resources:

- [Strings - Rust by Example](https://doc.rust-lang.org/rust-by-example/std/str.html)


#### Question #2 Do the unit tests pass?

This is the [Rubicon](https://en.wikipedia.org/wiki/Crossing_the_Rubicon) when it comes to builds. It is essential that a team keep their unit tests green, aka passing. Once you turn this check off because you "don't have time" you might as well turn in your keyboard and let the AI take over.

One of the key types of tests for any running system are what's called regression tests. [Wikipedia defines it](https://en.wikipedia.org/wiki/Regression_testing) like this:

> **Regression testing** (rarely, _non-regression testing_[[1]](https://en.wikipedia.org/wiki/Regression_testing#cite_note-1)) is re-running [functional](https://en.wikipedia.org/wiki/Functional_testing "Functional testing") and [non-functional tests](https://en.wikipedia.org/wiki/Non-functional_testing "Non-functional testing") to ensure that previously developed and tested software still performs as expected after a change.[[2]](https://en.wikipedia.org/wiki/Regression_testing#cite_note-2) If not, that would be called a _[regression](https://en.wikipedia.org/wiki/Software_regression "Software regression")_.

*OLD MAN VOICE:* Back in my day we had what were called Quality Assurance Engineers. These were people who went through your code to make sure that the things you promised to deliver  actually were, and that you didn't break any of the existing functionality to do so.

Good coverage from unit tests translates into an army of automated regression testers ready to descend upon your code at the push of a button.

> *And for the record, Quality Assurance is still an essential role in any software worth its salt. But with unit tests, they can spend their time doing what's called exploratory testing. The big flaw in unit tests are that they're a self fulfilling prophecy. You can only test the things you think of. A good QA dev can bring down a site in ways that [many would call unnatural](https://www.youtube.com/watch?v=GqcSXt1CQOQ).*
>
> STORY TIME: I was leading a team at a small rust belt financial institution. The consulting practice I was working for at the time  brought in a QA by the name of Paul Oaks. When we were ready to test out the front end of our application, he sat down and typed in an address. The site instantly self destructed. It took him less than one minute to destroy our application.
>
> So, I'm like, how the F' did you do that? Paul collected bizarre addresses that he knew would crap out at a lot of commercial address validation services. **RESPECT YOUR QA.**

#### Question #3 Do all the [Clippy](https://doc.rust-lang.org/clippy/) lints pass?

No, not that [Clippy]... Rust's linter Clippy.

Linting is a form of static code analysis that checks that you're not making certain kinks of programming decisions that make your code hard to maintain. Large codebases will generally establish a common set of linting configurations so that everyone is on the same page.

Linting is one of the great ways to keep a codebase clean, and consistent. It's easy to miss simple, stupid things in the rush to get things out the door. A good linter that's tied to a continuous integration tool like [GitHub Actions](https://github.com/features/actions) or [Jenkins](https://www.jenkins.io/)

For this kata, I've configured the linting at the top of the [src/lib.rs](https://github.com/devplaybooks/rust_worlds_simplest_kata/blob/main/src/lib.rs) file:

```rust
#![warn(clippy::pedantic)]  
  
pub struct Hello;
```

#### Question #4  Is the code properly [formatted](https://github.com/rust-lang/rustfmt) ?

The [fmt](https://github.com/rust-lang/rustfmt) check seems like a trivial one, but in large teams a difference of formatting can cause havoc on a large pull request.

#### Question #5 Do all the [Rustdoc lints] pass?

I LOVE LOVE LOVE how Rust handles documentation. One of the biggest challenges in writing software is keeping its documentation up to date. Code samples quickly become wrong, and rushed developers are rarely given enough time to make sure that the docs are up to date, especially within an organization with a predominance of what I call "neck breathers."

[Rustdoc lints](https://doc.rust-lang.org/rustdoc/lints.html) and [doc tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) eliminate almost all of this problem by catching references to code blocks that don't exist, and example code that doesn't compile or pass tests. This is HUGE! One of the biggest standoffs between developers and stakeholders is documentation. Stakeholders demand it, and devs point out that it is out of date as soon as it ships. Doc tests eliminate that issue. The linters check that you are pointing to the correct places in the code, and the compiler makes sure that any exxamples you include work against the actual codebase.

Thanks to GitHub [Actions](https://github.com/features/actions) you can automate all of those features so that they run every time you push up code so that everyone is always on the same page.

**MORAL:** *Automate the stupid stuff.*

-----

## Let's see how CoPilot did.

Wow, CoPilot fixed every test. Six of the GitHub continuous integration gates passed. The problem
is, that there are seven. _[Sad trombone sound](https://www.youtube.com/shorts/9inOVXhe14U)_

[CoPilot couldn't fix the clippy lints](https://github.com/folkengine/wsk_CoPilot2/actions/runs/31438082328/job/93617372018?pr=1#step:4:2):

```shell
Run cargo clippy -- -Dclippy::all -Dclippy::pedantic
  cargo clippy -- -Dclippy::all -Dclippy::pedantic
  shell: /usr/bin/bash -e {0}
  env:
    RUSTFLAGS: -Dwarnings
    CARGO_HOME: /home/runner/.cargo
    CARGO_INCREMENTAL: 0
    CARGO_TERM_COLOR: always
    Checking worlds_simplest_kata v0.1.2 (/home/runner/work/rust_worlds_simplest_kata/rust_worlds_simplest_kata)
error: this method could have a `#[must_use]` attribute
  --> src/lib.rs:18:12
   |
18 |     pub fn hello(name: String) -> String {
   |            ^^^^^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.97.0/index.html#must_use_candidate
   = note: `-D clippy::must-use-candidate` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(clippy::must_use_candidate)]`
help: add the attribute
   |
18 ~     #[must_use]
19 ~     pub fn hello(name: String) -> String {
   |

error: this argument is passed by value, but not consumed in the function body
  --> src/lib.rs:18:24
   |
18 |     pub fn hello(name: String) -> String {
   |                        ^^^^^^ help: consider changing the type to: `&str`
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.97.0/index.html#needless_pass_by_value
   = note: `-D clippy::needless-pass-by-value` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(clippy::needless_pass_by_value)]`

error: variables can be used directly in the `format!` string
  --> src/lib.rs:19:9
   |
19 |         format!("Hello, {}!", name)
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.97.0/index.html#uninlined_format_args
   = note: `-D clippy::uninlined-format-args` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(clippy::uninlined_format_args)]`
help: change this to
   |
19 -         format!("Hello, {}!", name)
19 +         format!("Hello, {name}!")
   |

error: this method could have a `#[must_use]` attribute
  --> src/lib.rs:22:12
   |
22 |     pub fn hello_world() -> String {
   |            ^^^^^^^^^^^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.97.0/index.html#must_use_candidate
help: add the attribute
   |
22 ~     #[must_use]
23 ~     pub fn hello_world() -> String {
   |

error: could not compile `worlds_simplest_kata` (lib) due to 4 previous errors
Error: Process completed with exit code 101.
```


[^1]: GitHub actions have become an [attack vector ](https://medium.com/@simardeep.oberoi/unveiling-github-actions-vulnerabilities-a-comprehensive-technical-guide-to-attack-vectors-and-6a26a83e9fb2)of late, so always be careful about what you run.
[^2]:   [Michael Sambol](https://www.youtube.com/@MichaelSambol) - [Heaps: Intro in 3 minutes](https://www.youtube.com/watch?v=0wPlzMU-k00)


