//! Replay a plan into a real git repository with `git` plumbing. Every
//! commit's parents come from the plan, so merges are two-parent commits and
//! branches are refs. No clock, no machine identity: SHAs are a pure function
//! of the plan.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;

use crate::{Line, RepoPlan, TreeState};

pub const EPOCH: i64 = 1_756_684_800; // 2026-09-01T00:00:00Z
pub const NAME: &str = "ImperialBower Bower";
pub const EMAIL: &str = "bower@imperialbower.example";

#[derive(Debug)]
pub struct ReplayReport {
    pub refs: BTreeMap<String, String>,
    pub commits: usize,
}

fn git(dir: &Path, seq: usize, args: &[&str], stdin: Option<&[u8]>) -> Result<String, String> {
    let stamp = format!("{} +0000", EPOCH + (seq as i64) * 60);
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(args)
        .env("GIT_AUTHOR_NAME", NAME).env("GIT_AUTHOR_EMAIL", EMAIL).env("GIT_AUTHOR_DATE", &stamp)
        .env("GIT_COMMITTER_NAME", NAME).env("GIT_COMMITTER_EMAIL", EMAIL).env("GIT_COMMITTER_DATE", &stamp)
        .env_remove("GIT_DIR").env_remove("GIT_WORK_TREE")
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("git: {e}"))?;
    if let Some(bytes) = stdin {
        child.stdin.take().expect("piped").write_all(bytes).map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Write a tree, recursing into directories via `mktree`.
fn write_tree(dir: &Path, tree: &TreeState) -> Result<String, String> {
    fn go(dir: &Path, entries: &BTreeMap<String, String>) -> Result<String, String> {
        // Group by first path segment.
        let mut files: BTreeMap<String, &String> = BTreeMap::new();
        let mut dirs: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for (path, body) in entries {
            match path.split_once('/') {
                None => { files.insert(path.clone(), body); }
                Some((head, rest)) => {
                    dirs.entry(head.to_string()).or_default().insert(rest.to_string(), body.clone());
                }
            }
        }
        let mut listing = String::new();
        for (name, body) in files {
            let sha = git(dir, 0, &["hash-object", "-w", "--stdin"], Some(body.as_bytes()))?;
            listing.push_str(&format!("100644 blob {sha}\t{name}\n"));
        }
        for (name, sub) in dirs {
            let sha = go(dir, &sub)?;
            listing.push_str(&format!("040000 tree {sha}\t{name}\n"));
        }
        git(dir, 0, &["mktree"], Some(listing.as_bytes()))
    }
    go(dir, &tree.0)
}

pub fn run(plan: &RepoPlan, dir: &Path) -> Result<ReplayReport, String> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    git(dir, 0, &["init", "-q", "-b", "main"], None)?;

    // Step 0 — scaffolding.
    let mut scaffold = TreeState::default();
    scaffold.0.insert("README.md".into(), "Generated from the book. Do not open PRs here.\n".into());
    let t0 = write_tree(dir, &scaffold)?;
    let c0 = git(dir, 0, &["commit-tree", &t0, "-m", "Initial commit — scaffolding"], None)?;
    let mut sha_of: BTreeMap<usize, String> = BTreeMap::new();
    sha_of.insert(0, c0.clone());
    let mut commits = 1;

    let mut heads: BTreeMap<String, String> = BTreeMap::new();
    heads.insert("main".into(), c0);

    for s in &plan.steps {
        let mut full = scaffold.clone();
        full.0.extend(s.tree.0.clone());
        let tree = write_tree(dir, &full)?;
        let mut args = vec!["commit-tree".to_string(), tree];
        for p in &s.parents {
            args.push("-p".into());
            args.push(sha_of[p].clone());
        }
        let msg = format!("{}\n\nBower-Step: failers/{:03}\nBower-Line: {}", s.msg, s.seq, s.line);
        args.push("-m".into());
        args.push(msg);
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let sha = git(dir, s.seq, &argv, None)?;
        commits += 1;
        sha_of.insert(s.seq, sha.clone());
        let head = match &s.line { Line::Main => "main".to_string(), Line::Branch(b) => b.clone() };
        heads.insert(head, sha.clone());
        let tag = s.tag();
        git(dir, s.seq, &["tag", "-a", "-m", &s.msg, &tag, &sha], None)?;
    }

    let mut refs = BTreeMap::new();
    for (name, sha) in &heads {
        git(dir, 0, &["update-ref", &format!("refs/heads/{name}"), sha], None)?;
        refs.insert(format!("refs/heads/{name}"), sha.clone());
    }
    for s in &plan.steps {
        refs.insert(format!("refs/tags/{}", s.tag()), sha_of[&s.seq].clone());
    }
    git(dir, 0, &["reset", "-q", "--hard", "main"], None)?;
    Ok(ReplayReport { refs, commits })
}

pub fn graph(dir: &Path) -> String {
    git(dir, 0, &["log", "--graph", "--oneline", "--all", "--decorate", "--date-order"], None).unwrap_or_default()
}
