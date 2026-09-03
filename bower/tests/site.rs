//! The site branch, end to end against `FakeForge`.
//!
//! No test here touches the network. The decisions — whether to publish, what
//! the gate says, whether anything was sent — are all answerable against a fake
//! that counts calls, which is the whole reason `Forge` exists.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bower::config::BookConfig;
use bower::forge::{FakeForge, RemoteState};
use bower::loader::BookLoader;
use bower::materialize::write_files;
use bower::publish::{site_fingerprint, site_marker, write_site_files};
use bower::push::{plan_push, PushPlan};
use bower::replay::{book_name, expected_tags, final_blobs, scaffolding};
use bower::status::{site_drift, SiteDrift};
use bower_core::prelude::{lock_text, plan, BookPlan, RepoPlan};

fn book_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("books")
        .join("hello-playbook")
}

fn scratch(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bower-sitetest-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn sample() -> (BookConfig, BookPlan, RepoPlan, String) {
    let cfg = BookConfig::load(&book_root()).unwrap();
    let book = BookLoader::new(&book_root()).load().unwrap();
    let resolved = plan(&book, &cfg.catalog()).unwrap();
    let fingerprint = site_fingerprint(&book, &lock_text(&resolved));
    let repo = resolved.repos[0].clone();
    (cfg, resolved, repo, fingerprint)
}

/// A built repository, as `bower build` would leave it.
fn built(case: &str, cfg: &BookConfig, p: &RepoPlan) -> PathBuf {
    let dir = scratch(case).join("repo");
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

/// A rendered book carrying `fingerprint`.
fn rendered(case: &str, fingerprint: &str) -> PathBuf {
    let dir = scratch(case).join("site");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<html/>").unwrap();
    write_site_files(&dir, &site_marker("hello-playbook", fingerprint)).unwrap();
    dir
}

#[test]
fn a_configured_book_plans_a_site_push() {
    let (cfg, _, repo, fp) = sample();
    let dir = built("configured", &cfg, &repo);
    let site = rendered("configured-site", &fp);

    let forge = FakeForge::new(RemoteState::Absent, None);
    let got = plan_push(&forge, &cfg, &fp, &repo, &dir, &site, &book_root()).unwrap();

    let PushPlan::Ready { site: Some(s), .. } = got else {
        panic!("expected a site push, got {got:?}");
    };
    assert_eq!(s.branch, "gh-pages");
    assert!(s.create, "the fake has no site branch yet");
    // Deciding is not doing.
    assert!(!forge.mutated(), "{:?}", forge.calls());
}

#[test]
fn a_book_with_no_site_branch_plans_none() {
    let (mut cfg, _, repo, fp) = sample();
    cfg.repos.get_mut("hello-playbook").unwrap().site_branch = None;
    let dir = built("unconfigured", &cfg, &repo);

    let forge = FakeForge::new(RemoteState::Absent, None);
    let got = plan_push(
        &forge,
        &cfg,
        &fp,
        &repo,
        &dir,
        Path::new("/no-site"),
        &book_root(),
    )
    .unwrap();

    assert!(matches!(got, PushPlan::Ready { site: None, .. }), "{got:?}");
    assert!(
        !forge.calls().iter().any(|c| c == "probe_branch"),
        "a book that ships no site must not ask about a branch: {:?}",
        forge.calls()
    );
}

#[test]
fn a_hand_built_site_is_refused_and_nothing_is_sent() {
    // The case that actually happened: a `gh-pages` created by hand, before
    // markers existed. Bower cannot tell it from a stranger's, and must not
    // overwrite it.
    let (cfg, _, repo, fp) = sample();
    let dir = built("handbuilt", &cfg, &repo);
    let site = rendered("handbuilt-site", &fp);

    let forge = FakeForge::new(RemoteState::Absent, None).with_site(RemoteState::HasContent, None);
    let got = plan_push(&forge, &cfg, &fp, &repo, &dir, &site, &book_root()).unwrap();

    let PushPlan::Blocked { reason, .. } = got else {
        panic!("expected Blocked, got {got:?}");
    };
    assert!(
        reason.contains(".bower-site"),
        "name the right file: {reason}"
    );
    assert!(
        !forge.mutated(),
        "somebody's site was touched: {:?}",
        forge.calls()
    );
}

#[test]
fn our_own_site_is_recognised_and_updated_in_place() {
    let (cfg, _, repo, fp) = sample();
    let dir = built("ours", &cfg, &repo);
    let site = rendered("ours-site", &fp);
    let marker = site_marker("hello-playbook", &fp);

    let forge =
        FakeForge::new(RemoteState::Absent, None).with_site(RemoteState::HasContent, Some(marker));
    let got = plan_push(&forge, &cfg, &fp, &repo, &dir, &site, &book_root()).unwrap();

    assert!(
        matches!(got, PushPlan::Ready { site: Some(ref s), .. } if !s.create),
        "an existing site of ours is updated, not created: {got:?}"
    );
}

#[test]
fn a_prose_edit_alone_makes_the_site_stale() {
    // The failure this EPIC exists to catch. The plan does not change, so the
    // lock and the repo both report in sync — and the site must not.
    let (_, resolved, _, _) = sample();
    let book = BookLoader::new(&book_root()).load().unwrap();
    let lock = lock_text(&resolved);

    let before = site_fingerprint(&book, &lock);
    let mut edited = book.clone();
    edited.chapters[1].text.push_str("\nAn added paragraph.\n");
    let after = site_fingerprint(&edited, &lock);

    assert_ne!(
        before, after,
        "prose changes the rendered book even when the plan is untouched"
    );

    let site = rendered("prose", &before);
    assert!(matches!(
        site_drift(&site, "hello-playbook", &after, true),
        SiteDrift::Stale { .. }
    ));
}

#[test]
fn the_rendered_book_carries_both_host_files() {
    let (_, _, _, fp) = sample();
    let site = rendered("hostfiles", &fp);
    assert!(
        site.join(".nojekyll").exists(),
        "Jekyll would eat the assets"
    );
    assert!(
        site.join(".bower-site").exists(),
        "the gate needs its marker"
    );
}

#[test]
fn link_templates_are_unaffected_by_the_site_branch() {
    // A guard against the site work leaking into the repo links.
    let (cfg, _, _, _) = sample();
    let links: BTreeMap<_, _> = cfg
        .repos
        .iter()
        .map(|(n, r)| (n.clone(), r.links.blob.clone()))
        .collect();
    assert!(links["hello-playbook"].as_ref().unwrap().contains("{tag}"));
}
