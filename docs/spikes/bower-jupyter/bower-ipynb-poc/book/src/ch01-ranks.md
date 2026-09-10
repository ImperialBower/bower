# Chapter 1 — Ranks, cards, and the first controlled failure

A poker library starts with a rank. Not a card — a rank. Suits are four bits of
bookkeeping; ranks carry the arithmetic that makes Cactus Kev's evaluator fast.
Each rank owns a **prime** (so a product of five primes identifies a hand's rank
multiset) and a **bit** (so a 13-bit mask identifies straights and flushes).

<!-- bower repo="failers" step="rank-enum" file="src/rank.rs" msg="ch01: Introduce Rank" -->
```rust
# #![allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Rank {
    Deuce, Trey, Four, Five, Six, Seven, Eight,
    Nine, Ten, Jack, Queen, King, Ace,
}

impl Rank {
    const PRIMES: [u32; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

    pub fn prime(self) -> u32 { Self::PRIMES[self as usize] }
    pub fn bits(self) -> u32 { 1 << (16 + self as u32) }
}

// bower:begin from_char
// bower:end from_char
```

The numbers are the point, so look at them. In the notebook edition the cell
below is live: change `ACE` to `DEUCE`, re-run, and watch the prime and the bit
move together.

<!-- bower repo="failers" notebook="play" -->
```python
from pkcore import Rank

r = Rank.ACE
print("prime:", r.prime())      # 41 — the largest of the 13 rank primes
print("bit:  ", bin(r.bits()))  # one bit, high in the word
```

## The failure you are supposed to hit

Now parse a rank from a character. The obvious first draft is a `match` with
the thirteen legal letters and nothing else. Write it. Compile it. It does not
compile — and the compiler's complaint is the whole lesson of this book: a
non-exhaustive match is a *controlled* failure, caught at the earliest possible
moment, for free.

<!-- bower repo="failers" step="rank-from-char-fail" file="src/rank.rs" op="region" region="from_char" expect="compile_fail" msg="ch01: Rank::from(char), the version that does not compile" -->
```rust
impl From<char> for Rank {
    fn from(c: char) -> Self {
        match c {
            'A' => Rank::Ace,   'K' => Rank::King,  'Q' => Rank::Queen,
            'J' => Rank::Jack,  'T' => Rank::Ten,   '9' => Rank::Nine,
            '8' => Rank::Eight, '7' => Rank::Seven, '6' => Rank::Six,
            '5' => Rank::Five,  '4' => Rank::Four,  '3' => Rank::Trey,
            '2' => Rank::Deuce,
        }
    }
}
```

`bower verify` asserts that this step fails to compile. The same beat crosses
the language boundary: pkcore's parser refuses garbage with a `ValueError`,
and the notebook edition asserts that the exception *happens*.

<!-- bower repo="failers" notebook="play" expect="raises" -->
```python
from pkcore import Card

Card.parse("Zz")   # not a card — this cell is *supposed* to raise
```

## The fix, and the brute-force test

The fix is one line — a fallible constructor instead of an infallible one — and
then a test that walks every legal character, because thirteen cases is small
enough to enumerate and enumeration beats cleverness.

<!-- bower repo="failers" step="rank-from-char" file="src/rank.rs" op="region" region="from_char" msg="ch01: Rank::try_from(char), tested the brute-force way" -->
```rust
impl TryFrom<char> for Rank {
    type Error = char;
    fn try_from(c: char) -> Result<Self, char> {
        Ok(match c.to_ascii_uppercase() {
            'A' => Rank::Ace,   'K' => Rank::King,  'Q' => Rank::Queen,
            'J' => Rank::Jack,  'T' => Rank::Ten,   '9' => Rank::Nine,
            '8' => Rank::Eight, '7' => Rank::Seven, '6' => Rank::Six,
            '5' => Rank::Five,  '4' => Rank::Four,  '3' => Rank::Trey,
            '2' => Rank::Deuce,
            other => return Err(other),
        })
    }
}
```

<!-- bower repo="failers" step="rank-from-char" file="src/rank.rs" op="append" -->
```rust

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // bower:show
    #[rstest]
    #[case('A', Rank::Ace)] #[case('K', Rank::King)] #[case('Q', Rank::Queen)]
    #[case('J', Rank::Jack)] #[case('T', Rank::Ten)] #[case('9', Rank::Nine)]
    #[case('8', Rank::Eight)] #[case('7', Rank::Seven)] #[case('6', Rank::Six)]
    #[case('5', Rank::Five)] #[case('4', Rank::Four)] #[case('3', Rank::Trey)]
    #[case('2', Rank::Deuce)]
    fn try_from_char(#[case] c: char, #[case] expected: Rank) {
        assert_eq!(Rank::try_from(c), Ok(expected));
        assert_eq!(Rank::try_from(c.to_ascii_lowercase()), Ok(expected));
    }

    #[test]
    fn try_from_char_rejects_garbage() {
        assert_eq!(Rank::try_from('Z'), Err('Z'));
    }
    // bower:show end
}
```

With a parser in hand, a card is a rank, a suit, and the packed `u32` that
pkcore actually stores. Read the four bytes: rank bit, suit flag, rank index,
rank prime.

<!-- bower repo="failers" notebook="play" -->
```python
from pkcore import Card

c = Card.parse("As")
print(c, "→", c.as_u32())
print(c.bit_string())          # |bbbbbbbb|bbbbbbbb|SHDCrrrr|xxpppppp|
print("rank prime:", c.get_rank_prime())
```

## Where this is going

Thirteen primes and thirteen bits are enough to evaluate a hand, and enough to
count outs. Here is the destination, live: two players, a turn, and every
river card that would change who wins.

<!-- bower repo="failers" notebook="play" -->
```python
from pkcore import HoleCards, Board, Game, Outs

hc = HoleCards.parse("As Kh 8d Kc")     # P1: A♠ K♥   P2: 8♦ K♣
board = Board.parse("Ac 8h 7h 9s")
outs = Outs.from_case_evals(Game(hc, board).turn_case_evals())

print("P1 outs:", outs.len_for_player(1))
print("P2 outs:", outs.len_for_player(2), "→", outs.get(2))
```
