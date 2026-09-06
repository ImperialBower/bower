# Rank, and the match that would not compile

A deck of cards is a good place to start a library, because a card is small
enough to hold in your head and stubborn enough to punish sloppiness. A card has
a rank and a suit. This chapter builds the rank.

It also does something you may not be used to in a book: it commits code that
does not compile, on purpose, and leaves it there. That commit is tagged. You
can check it out. You can run `cargo check` inside it and watch it fail.

That is the argument of this whole book. A failure you have never seen is a
failure you do not understand. So we are going to look at them.

## The rank itself

<!-- bower repo="rust4failures" step="rank-enum" file="src/rank.rs" msg="feat: the Rank enum" -->

```rust
// bower:begin rank
/// The rank of a playing card.
///
/// The discriminants are the card's value in play, which is why the ace is
/// fourteen rather than one. `BLANK` is the zero value: it is what you get
/// when a rank is asked for and none is there.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Rank {
    ACE = 14,
    KING = 13,
    QUEEN = 12,
    JACK = 11,
    TEN = 10,
    NINE = 9,
    EIGHT = 8,
    SEVEN = 7,
    SIX = 6,
    FIVE = 5,
    FOUR = 4,
    TREY = 3,
    DEUCE = 2,
    #[default]
    BLANK = 0,
}
// bower:end rank

// bower:begin from_char
// bower:end from_char
```

<!-- bower repo="rust4failures" step="rank-enum" file="src/lib.rs" op="region" region="mods" -->

```rust
pub mod rank;

```

A module that nothing declares is never compiled, so the declaration and the
file it points at belong in the same commit. Note the blank line at the end of
that block: it belongs to the region, not to the file around it, so that
emptying the region again would not leave two blank lines behind.

The names are shouted — `ACE`, not `Ace`. That is a break with Rust's usual
style, and you would expect `non_camel_case_types` to complain. It does not:
that lint stays quiet for a name that is already entirely uppercase, and clippy
at its most pedantic has nothing to add either. A style rule you assumed was
enforced turns out not to be, which is worth knowing before you lean on it.

## The match that would not compile

Now the interesting part. A rank should be constructible from the character that
names it, so that `'K'` becomes `Rank::KING`. The obvious way to write that is a
`match`, and the obvious `match` is wrong.

<!-- bower repo="rust4failures" step="from-char-broken" file="src/rank.rs" op="region" region="from_char" expect="compile_fail" msg="feat: Rank::from(char) (does not compile)" -->

```rust
impl From<char> for Rank {
    fn from(c: char) -> Self {
        match c {
            'A' | 'a' => Rank::ACE,
            'K' | 'k' => Rank::KING,
            'Q' | 'q' => Rank::QUEEN,
            'J' | 'j' => Rank::JACK,
            'T' | 't' | '0' => Rank::TEN,
            '9' => Rank::NINE,
            '8' => Rank::EIGHT,
            '7' => Rank::SEVEN,
            '6' => Rank::SIX,
            '5' => Rank::FIVE,
            '4' => Rank::FOUR,
            '3' => Rank::TREY,
            '2' => Rank::DEUCE,
        }
    }
}
```

Every rank is listed. Every card in the deck is covered. The code is still
rejected, and the message is the lesson:

```console
$ cargo check
error[E0004]: non-exhaustive patterns: `'\0'..='/'`, `'1'`, `':'..='@'` and 9 more not covered
```

`char` is not "the characters on a playing card". `char` is every Unicode scalar
value, and the compiler will not let you pretend otherwise. It is not asking
what a card is. It is asking what happens when someone hands you a `'z'`.

`expect="compile_fail"` on the directive above declares that this step is
supposed to be rejected. That is a promise the toolchain checks: when Bower
replays this book it runs `cargo check` at this step and treats a *successful*
build as the error. The failure is not an accident that survived into print. It
is an asserted fact about this commit.

<!-- bower repo="rust4failures" exercise="Make this compile" -->
```markdown
The compiler has named every character it is not told about. Each one needs an
answer — not just the thirteen on a card.

- The smallest fix is one more arm. Which pattern, and what does it return?
- Could `'z'` be an *error* instead of a rank? What would `from` have to
  become for that to be true?
```

## Answering the question

The compiler asked what happens to a `'z'`. `BLANK` is the answer, and the whole
reason `BLANK` exists.

<!-- bower repo="rust4failures" step="from-char" file="src/rank.rs" op="region" region="from_char" msg="fix: Rank::from(char) answers for every char" -->

```rust
impl From<char> for Rank {
    fn from(c: char) -> Self {
        match c {
            'A' | 'a' => Rank::ACE,
            'K' | 'k' => Rank::KING,
            'Q' | 'q' => Rank::QUEEN,
            'J' | 'j' => Rank::JACK,
            'T' | 't' | '0' => Rank::TEN,
            '9' => Rank::NINE,
            '8' => Rank::EIGHT,
            '7' => Rank::SEVEN,
            '6' => Rank::SIX,
            '5' => Rank::FIVE,
            '4' => Rank::FOUR,
            '3' => Rank::TREY,
            '2' => Rank::DEUCE,
            // bower:show
            _ => Rank::BLANK,
            // bower:show end
        }
    }
}
```

One line. The block above holds the whole function, because the repository needs
the whole function, but only the new line was printed on the page. That is what
the `bower:show` markers do, and they never reach the repository.

## Telling the hero's story

A conversion is a claim about every input, so the tests are about every input.
Not a representative sample. Every one.

<!-- bower repo="rust4failures" step="from-char-tested" file="Cargo.toml" op="region" region="dev-dependencies" msg="test: from(char), brute force" -->

```toml
[dev-dependencies]
rstest = "0.23"

```

<!-- bower repo="rust4failures" step="from-char-tested" file="src/rank.rs" op="append" -->

```rust

#[cfg(test)]
#[allow(non_snake_case)]
mod rank_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case('A', Rank::ACE)]
    #[case('a', Rank::ACE)]
    #[case('K', Rank::KING)]
    #[case('k', Rank::KING)]
    #[case('Q', Rank::QUEEN)]
    #[case('q', Rank::QUEEN)]
    #[case('J', Rank::JACK)]
    #[case('j', Rank::JACK)]
    #[case('T', Rank::TEN)]
    #[case('t', Rank::TEN)]
    #[case('0', Rank::TEN)]
    #[case('9', Rank::NINE)]
    #[case('8', Rank::EIGHT)]
    #[case('7', Rank::SEVEN)]
    #[case('6', Rank::SIX)]
    #[case('5', Rank::FIVE)]
    #[case('4', Rank::FOUR)]
    #[case('3', Rank::TREY)]
    #[case('2', Rank::DEUCE)]
    #[case('_', Rank::BLANK)]
    #[case(' ', Rank::BLANK)]
    #[case('z', Rank::BLANK)]
    fn from__char(#[case] input: char, #[case] expected: Rank) {
        assert_eq!(expected, Rank::from(input));
    }
}
```

Two conventions worth naming, because they run through the rest of this book.

The test is called `from__char`, with two underscores, which is not idiomatic
Rust and is why the module allows `non_snake_case`. The doubled underscore reads
as "the tests for `from`, the `char` ones". When a test fails at three in the
morning, the name should tell you where to stand before you have read a line of
its body.

And the cases are exhaustive on purpose — `'z'` and `' '` and `'_'` sit in the
same list as the ace. The negative cases are not an afterthought bolted on at
the bottom. They are the reason the list exists at all. The compiler already
made us answer for them once. The tests hold us to the answer.

## What is in the repository now

Five commits, each one tagged and checkoutable. One of them does not build, and
that is recorded as a fact rather than hidden as an embarrassment.

That is the shape of every chapter from here.
