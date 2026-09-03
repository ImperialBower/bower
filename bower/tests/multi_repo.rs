//! Books that feed more than one repository.
//!
//! `bower` has always looped over repos, but no book exercised it, so this was
//! untested rather than proven (`docs/TECHNICAL_DEBT.md`). Two bugs lived in
//! that gap, both found in review.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bower-multirepo-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A book feeding two repos. `alpha-only` exists solely in `alpha`, so a
/// `--step alpha-only` must not be judged against `beta`'s plan.
fn two_repo_book(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("bower.toml"),
        concat!(
            "[book]\nepoch = 2026-09-01T00:00:00Z\n\n",
            "[identity]\nname = \"T\"\nemail = \"t@example.invalid\"\n\n",
            "[repos.alpha]\ncheck = \"true\"\nverify = \"true\"\n\n",
            "[repos.beta]\ncheck = \"true\"\nverify = \"true\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("src").join("SUMMARY.md"),
        "# Summary\n\n- [One](ch01.md)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src").join("ch01.md"),
        concat!(
            "# Two repos\n\n",
            "<!-- bower repo=\"alpha\" step=\"alpha-only\" file=\"src/lib.rs\" -->\n\n",
            "```rust\npub fn a() {}\n```\n\n",
            "<!-- bower repo=\"beta\" step=\"beta-only\" file=\"src/lib.rs\" -->\n\n",
            "```rust\npub fn b() {}\n```\n",
        ),
    )
    .unwrap();
}

fn bower(book: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book)
        .args(args)
        .output()
        .expect("the bower binary must run")
}

#[test]
fn verify_a_step_that_lives_in_only_one_repo() {
    // `alpha-only` is not in `beta`'s plan, and `beta` sorts first. Judging the
    // step against every repo's plan makes a valid request fail.
    let root = scratch("verify-step");
    let book = root.join("book");
    two_repo_book(&book);

    let out = bower(
        &book,
        &[
            "verify",
            "--step",
            "alpha-only",
            "--work",
            root.join("w").to_str().unwrap(),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "a step present in one repo must verify:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("alpha-only"), "{stdout}");
}

#[test]
fn a_step_in_no_repo_is_still_an_error() {
    // The counterweight: skipping repos that lack the step must not turn a
    // typo into a silent success.
    let root = scratch("verify-typo");
    let book = root.join("book");
    two_repo_book(&book);

    let out = bower(
        &book,
        &[
            "verify",
            "--step",
            "no-such-step",
            "--work",
            root.join("w").to_str().unwrap(),
        ],
    );
    assert!(!out.status.success(), "a typo must fail");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no-such-step"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn plan_with_an_unknown_repo_is_an_error() {
    // `build`, `status`, and `push` all refuse an unmatched `--repo`. `plan`
    // and `verify` printed nothing and exited 0, which reads as success.
    let root = scratch("plan-typo");
    let book = root.join("book");
    two_repo_book(&book);

    let out = bower(&book, &["plan", "--repo", "gamma"]);
    assert!(!out.status.success(), "an unmatched --repo must fail");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no repo matched"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn build_gives_each_repo_its_own_directory() {
    let root = scratch("build");
    let book = root.join("book");
    two_repo_book(&book);
    let out_dir = root.join("out");

    let out = bower(&book, &["build", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out_dir.join("alpha").join(".git").is_dir(), "alpha missing");
    assert!(out_dir.join("beta").join(".git").is_dir(), "beta missing");
    assert!(out_dir.join("alpha").join("src").join("lib.rs").exists());
}
