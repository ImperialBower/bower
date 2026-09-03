//! `bower push` — the command, and the guard.
//!
//! **No test here touches the network, ever.** The sample book declares no
//! `github` key, so driving the binary can only reach the "does not publish"
//! path; everything else is exercised through `FakeForge`, which counts calls
//! and sends nothing.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use bower::config::BookConfig;
use bower::forge::{FakeForge, RemoteState};
use bower::loader::BookLoader;
use bower::materialize::write_files;
use bower::push::{plan_push, PushPlan};
use bower::replay::{book_name, expected_tags, final_blobs, scaffolding};
use bower::trailers::marker_line;
use bower_core::prelude::{plan, BookPlan, RepoPlan};

fn book_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("books")
        .join("hello-playbook")
}

fn scratch(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bower-pushtest-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn bower(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book_root())
        .args(args)
        .output()
        .expect("the bower binary must run")
}

fn book_plan_of(p: &RepoPlan) -> BookPlan {
    BookPlan {
        repos: vec![p.clone()],
    }
}

fn sample_plan() -> (BookConfig, RepoPlan) {
    let cfg = BookConfig::load(&book_root()).unwrap();
    let book = BookLoader::new(&book_root()).load().unwrap();
    let mut resolved = plan(&book, &cfg.catalog()).unwrap();
    (cfg, resolved.repos.remove(0))
}

/// A repository shaped like `bower build`'s output, without running git.
fn built(case: &str, p: &RepoPlan, cfg: &BookConfig) -> PathBuf {
    let dir = scratch(case);
    std::fs::create_dir_all(&dir).unwrap();
    let tags = dir.join(".git").join("refs").join("tags");
    std::fs::create_dir_all(&tags).unwrap();
    for tag in expected_tags(p) {
        std::fs::write(tags.join(tag), "0\n").unwrap();
    }
    let name = book_name(&book_root(), &p.repo.0);
    let scaffold = scaffolding(cfg, &book_root(), &p.repo.0).unwrap();
    write_files(&dir, &final_blobs(p, &name, cfg.site.as_deref(), &scaffold)).unwrap();
    dir
}

/// A copy of the sample book with its `github` key removed.
///
/// The test below owns its fixture rather than leaning on the sample book's
/// configuration: the sample now publishes to a real remote, and a test that
/// asserts what it does *not* declare breaks the day someone configures it —
/// which is exactly what happened on 2 September 2026.
fn book_that_does_not_publish(case: &str) -> PathBuf {
    let dir = scratch(case);
    copy_dir(&book_root(), &dir);
    let toml = dir.join("bower.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    let stripped: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("github ="))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&toml, stripped).unwrap();
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap();
        if name == "book" {
            continue;
        }
        let target = to.join(name);
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

#[test]
fn a_book_without_a_github_key_publishes_nothing_and_says_so() {
    let book = book_that_does_not_publish("no-github-key");
    let out = Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(&book)
        .args(["push", "-o", "/nonexistent"])
        .output()
        .expect("the bower binary must run");

    assert!(
        out.status.success(),
        "a book that does not publish is normal: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("does not publish it"), "{text}");
}

#[test]
fn the_sample_book_now_declares_a_remote() {
    // The counterweight, and the thing the old test was accidentally asserting.
    let cfg = BookConfig::load(&book_root()).unwrap();
    assert_eq!(
        cfg.repos["hello-playbook"].github.as_deref(),
        Some("abstecker/hello-playbook")
    );
}

#[test]
fn dry_run_is_the_default_and_no_flag_overrides_the_guard() {
    // Exit criterion 5, made a test rather than a promise: the help text is the
    // whole surface a user can reach, and it must offer no way past the gate.
    let out = bower(&["push", "--help"]);
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(text.contains("--execute"), "{text}");
    for escape in ["--force", "--override", "--skip", "--no-verify", "--yes"] {
        assert!(
            !text.contains(escape),
            "`{escape}` would be a way around the guard: {text}"
        );
    }
}

#[test]
fn a_remote_that_is_not_ours_is_refused_and_nothing_is_sent() {
    // The assertion that matters: not that a refusal was printed, but that the
    // forge was never asked to change anything.
    let (cfg, p) = sample_plan();
    let mut cfg = cfg;
    cfg.repos.get_mut("hello-playbook").unwrap().github = Some("ImperialBower/pkcore".to_string());
    let dir = built("not-ours", &p, &cfg);

    let forge = FakeForge::new(RemoteState::HasContent, None);
    let got = plan_push(
        &forge,
        &cfg,
        &book_plan_of(&p),
        &p,
        &dir,
        Path::new("/no-site"),
        &book_root(),
    )
    .unwrap();

    let PushPlan::Blocked { reason, .. } = got else {
        panic!("expected Blocked, got {got:?}");
    };
    assert!(reason.contains("ImperialBower/pkcore"), "{reason}");
    assert!(!forge.mutated(), "{:?}", forge.calls());
}

#[test]
fn our_own_remote_is_ready_and_the_dry_run_still_sends_nothing() {
    let (cfg, p) = sample_plan();
    let mut cfg = cfg;
    cfg.repos.get_mut("hello-playbook").unwrap().github =
        Some("ImperialBower/hello-playbook".to_string());
    // This test is about the code branch. Say so, rather than inheriting
    // whatever the live sample book happens to declare today.
    cfg.repos.get_mut("hello-playbook").unwrap().site_branch = None;
    let dir = built("ours", &p, &cfg);

    let steps = format!("# Steps\n\n{}\n", marker_line("hello-playbook"));
    let forge = FakeForge::new(RemoteState::HasContent, Some(steps));
    let got = plan_push(
        &forge,
        &cfg,
        &book_plan_of(&p),
        &p,
        &dir,
        Path::new("/no-site"),
        &book_root(),
    )
    .unwrap();

    let PushPlan::Ready { tags, create, .. } = got else {
        panic!("expected Ready, got {got:?}");
    };
    assert_eq!(tags, 26, "20 step tags plus 6 chapter-end tags");
    assert!(!create, "the remote already exists");
    // Deciding is not doing.
    assert!(!forge.mutated(), "{:?}", forge.calls());
}
