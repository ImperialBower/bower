# Try it on a branch

Every chapter so far has been one straight line of commits. Real work is not.
You try something, it fails, and you keep the attempt anyway, because the
failure is the lesson. This chapter tries two things on two branches. One fails
and stays where it is. The other works, and is merged.

## An experiment that fails

<!-- bower repo="hello-playbook" step="shout" branch="try/shout" file="src/lib.rs" op="region" region="greet" expect="test_fail" msg="experiment: shout the greeting" -->

```rust
/// Build a greeting for `name`, ignoring stray whitespace — loudly.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("HELLO, {}!", name.trim().to_uppercase())
}
```

`branch="try/shout"` puts this commit on a branch of its own, forked from the
last commit on `main`. The tests from chapter 4 still expect `Hello, world!`,
so the step claims `expect="test_fail"`, and `bower verify` holds it to that.

The branch is never merged. It stays in the repository as a state you can
check out and poke at:

```sh
git checkout try/shout
cargo test
```

<!-- bower repo="hello-playbook" branch="try/shout" pr="Shout the greeting" -->

```markdown
Louder is not better when the tests pin the exact words. Left open, on
purpose: this is the attempt that did not work.
```

A `pr=` directive declares a pull request for a branch, and its fence is the
request's description. The repository lists every one in `PULLS.md`.

## A feature on a branch

<!-- bower repo="hello-playbook" step="greet-many" branch="feat/greet-many" file="src/lib.rs" op="region" region="greet" msg="feat: greet_all(), one greeting per name" -->

```rust
/// Build a greeting for `name`, ignoring stray whitespace.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name.trim())
}

/// One greeting per name, in order.
#[must_use]
pub fn greet_all(names: &[&str]) -> Vec<String> {
    names.iter().copied().map(greet).collect()
}
```

A second branch, forked from the same commit on `main`. It rewrites the same
region the failed experiment did, and that is fine: the two branches never
meet.

<!-- bower repo="hello-playbook" step="greet-many-test" branch="feat/greet-many" file="src/lib.rs" op="region" region="tests" msg="test: greet_all keeps the order" -->

```rust
#[cfg(test)]
mod tests {
    use super::{greet, greet_all};

    #[test]
    fn greet_uses_the_name() {
        assert_eq!(greet("world"), "Hello, world!");
    }

    #[test]
    fn greet_ignores_stray_whitespace() {
        assert_eq!(greet("  world  "), "Hello, world!");
    }

    #[test]
    fn greet_all_keeps_the_order() {
        assert_eq!(greet_all(&["a", "b"]), ["Hello, a!", "Hello, b!"]);
    }
}
```

## Meanwhile, on main

<!-- bower repo="hello-playbook" step="changelog" file="CHANGELOG.md" msg="docs: start a changelog" -->

```markdown
# Changelog

## Unreleased

- CI runs the gate on every push and every pull request.
```

`main` moves on without the branch. This commit knows nothing about
`greet_all`.

## The merge

<!-- bower repo="hello-playbook" step="merge-greet-many" merge="feat/greet-many" op="none" msg="merge: feat/greet-many" -->

`merge="feat/greet-many"` makes this a merge commit on `main` with two parents:
the changelog, and the branch's last commit. Bower builds it the same way every
time, by replaying the branch's blocks over `main` as it stands. The two sides
changed different files, so there is nothing to resolve. Had they both
rewritten the same region, the book would stop at plan time and ask for a
resolution on this step.
