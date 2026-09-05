//! Turning a `RepoPlan` into a real git repository.
//!
//! Replay always starts from an empty tree and replays the whole plan
//! (spec § 5.1). Nothing here consults a clock, a machine identity, or an
//! environment variable: every commit's author, committer, and both timestamps
//! come from `bower.toml`, which is what makes two runs produce identical SHAs.

use std::fmt;
use std::path::{Path, PathBuf};

use bower_core::prelude::{PlannedStep, RepoPlan};
use gix::ObjectId;
use gix::bstr::BStr;
use gix::object::tree::EntryKind;
use gix::objs::Kind;
use gix::refs::transaction::PreviousValue;
use time::Duration;

use crate::config::BookConfig;
use crate::materialize::{Blobs, blobs_of, check_all, read_dir_recursive, write_files};
use crate::trailers;

/// The branch every generated repository uses. Fixed, not inherited from the
/// machine's `init.defaultBranch`, which would otherwise vary per developer.
/// The branch every generated repository uses. Public because `push` sends it
/// and must not name it independently.
pub const BRANCH: &str = "refs/heads/main";

pub struct Replayer<'a> {
    pub config: &'a BookConfig,
    pub book_root: &'a Path,
    pub out_dir: &'a Path,
}

/// What one replay produced.
#[derive(Debug)]
pub struct ReplayReport {
    pub repo: String,
    pub head: ObjectId,
    pub commits: usize,
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub enum ReplayError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Git(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Git(e) => write!(f, "git: {e}"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Wrap any `gix` error without enumerating a dozen error types that all mean
/// "the object database refused this".
fn git<E: std::error::Error + Send + Sync + 'static>(e: E) -> ReplayError {
    ReplayError::Git(Box::new(e))
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> ReplayError + '_ {
    move |source| ReplayError::Io {
        path: path.to_path_buf(),
        source,
    }
}

impl Replayer<'_> {
    /// Replay `plan` into `out_dir`, from scratch.
    ///
    /// # Errors
    ///
    /// Returns [`ReplayError`] if the output directory cannot be prepared, the
    /// template cannot be read, or git refuses an object or reference.
    pub fn run(&self, plan: &RepoPlan) -> Result<ReplayReport, ReplayError> {
        let repo_name = plan.repo.0.clone();
        // The book's own name, for `Book-Source` trailers. It is the book
        // directory's name, which is what a reader sees in a URL.
        let book_name = book_name(self.book_root, &repo_name);

        // From scratch, always: an existing directory is replaced, never merged.
        if self.out_dir.exists() {
            std::fs::remove_dir_all(self.out_dir).map_err(io(self.out_dir))?;
        }
        std::fs::create_dir_all(self.out_dir).map_err(io(self.out_dir))?;

        let repo = gix::init(self.out_dir).map_err(git)?;
        // `gix::init` honours the machine's `init.defaultBranch`. Pin it.
        let head = repo.path().join("HEAD");
        std::fs::write(&head, format!("ref: {BRANCH}\n")).map_err(io(&head))?;

        let mut parent: Option<ObjectId> = None;
        let mut commits = 0_usize;
        let mut tags = Vec::new();

        // Step 0 — scaffolding the book never shows.
        let scaffolding =
            scaffolding(self.config, self.book_root, &repo_name).map_err(|source| {
                ReplayError::Io {
                    path: self.book_root.to_path_buf(),
                    source,
                }
            })?;
        if !scaffolding.is_empty() {
            let tree = Self::write_tree(&repo, &scaffolding)?;
            let msg = trailers::scaffolding_message(&repo_name);
            let id = self.commit(&repo, 0, &msg, tree, parent)?;
            parent = Some(id);
            commits += 1;
        }

        let last_seq = plan.steps.last().map(|s| s.seq);
        let final_blobs = final_blobs(plan, &book_name, self.config.site.as_deref(), &scaffolding);

        for step in &plan.steps {
            // Each step's tree *replaces* the tree, so the scaffolding has to
            // be laid down under every one of them. Building a step from the
            // kernel's paths alone silently deletes the licences that step 0
            // had just added — which is exactly what it used to do.
            let mut blobs = scaffolding.clone();
            blobs.extend(blobs_of(&step.tree));
            if Some(step.seq) == last_seq {
                blobs.clone_from(&final_blobs);
            }
            let tree = Self::write_tree(&repo, &blobs)?;
            let msg =
                trailers::commit_message(step, &book_name, self.config.site.as_deref(), &repo_name);
            let id = self.commit(&repo, step.seq, &msg, tree, parent)?;
            parent = Some(id);
            commits += 1;

            let tag = step.tag();
            self.tag(&repo, &tag, id, step.seq, &step.msg)?;
            tags.push(tag);
        }

        for (stem, seq, id) in chapter_ends(plan, &repo, parent)? {
            let name = format!("{stem}-end");
            self.tag(&repo, &name, id, seq, &format!("end of {stem}"))?;
            tags.push(name);
        }

        let head_id = parent.unwrap_or_else(|| ObjectId::empty_tree(repo.object_hash()));
        self.write_worktree(&repo, &final_blobs)?;

        Ok(ReplayReport {
            repo: repo_name,
            head: head_id,
            commits,
            tags,
        })
    }

    /// Git's own time format: seconds since the epoch, then a UTC offset. Held
    /// at `+0000` deliberately — a local offset would change every SHA.
    fn stamp(&self, seq: usize) -> String {
        let at = self.config.epoch + Duration::minutes(i64::try_from(seq).unwrap_or(i64::MAX));
        format!("{} +0000", at.unix_timestamp())
    }

    fn commit(
        &self,
        repo: &gix::Repository,
        seq: usize,
        message: &str,
        tree: ObjectId,
        parent: Option<ObjectId>,
    ) -> Result<ObjectId, ReplayError> {
        let when = self.stamp(seq);
        let who = gix::actor::SignatureRef {
            name: BStr::new(self.config.identity.name.as_bytes()),
            email: BStr::new(self.config.identity.email.as_bytes()),
            time: &when,
        };
        let parents: Vec<ObjectId> = parent.into_iter().collect();
        // Author and committer are the same signature: there is only ever one
        // actor here, and two differing times would be two ways to drift.
        let id = repo
            .commit_as(who, who, BRANCH, message, tree, parents)
            .map_err(git)?;
        Ok(id.detach())
    }

    fn tag(
        &self,
        repo: &gix::Repository,
        name: &str,
        target: ObjectId,
        seq: usize,
        message: &str,
    ) -> Result<(), ReplayError> {
        let when = self.stamp(seq);
        let tagger = gix::actor::SignatureRef {
            name: BStr::new(self.config.identity.name.as_bytes()),
            email: BStr::new(self.config.identity.email.as_bytes()),
            time: &when,
        };
        repo.tag(
            name,
            target,
            Kind::Commit,
            Some(tagger),
            message,
            PreviousValue::Any,
        )
        .map_err(git)?;
        Ok(())
    }

    fn write_tree(repo: &gix::Repository, blobs: &Blobs) -> Result<ObjectId, ReplayError> {
        // `gix` happens to reject `..` as a tree filename, which masked this
        // for `build` while `verify` — which writes files with no git in the
        // loop — was wide open. Refuse here explicitly, so the guarantee is
        // ours and survives a change of git backend.
        check_all(blobs).map_err(|source| ReplayError::Io {
            path: PathBuf::from("<book>"),
            source,
        })?;
        let mut editor = repo
            .edit_tree(ObjectId::empty_tree(repo.object_hash()))
            .map_err(git)?;
        for (path, (bytes, exec)) in blobs {
            let oid = repo.write_blob(bytes).map_err(git)?;
            let kind = if *exec {
                EntryKind::BlobExecutable
            } else {
                EntryKind::Blob
            };
            editor
                .upsert(path.as_str(), kind, oid.detach())
                .map_err(git)?;
        }
        Ok(editor.write().map_err(git)?.detach())
    }

    /// Materialize the final step onto disk and write a matching index, so the
    /// generated repository is one a reader can `cd` into and `git status`.
    fn write_worktree(
        &self,
        repo: &gix::Repository,
        final_blobs: &Blobs,
    ) -> Result<(), ReplayError> {
        if final_blobs.is_empty() {
            return Ok(());
        }
        write_files(self.out_dir, final_blobs).map_err(io(self.out_dir))?;

        let tree = repo
            .head_commit()
            .map_err(git)?
            .tree_id()
            .map_err(git)?
            .detach();
        let mut index = repo.index_from_tree(&tree).map_err(git)?;
        index
            .write(gix::index::write::Options::default())
            .map_err(git)?;
        Ok(())
    }
}

/// The book's own name, as `Book-Source` trailers and `STEPS.md` print it: the
/// book directory's name, which is what a reader sees in a URL.
///
/// Public because `status` must expect the same `STEPS.md` that `replay` wrote,
/// and a second guess at the book's name would produce a file that differs by a
/// word and reports as drift forever.
///
/// The path is made absolute first, because one directory can be spelled more
/// than one way and every spelling must yield the same name. `--book` defaults
/// to `.`, which has no last component at all: taken literally it named every
/// book `"book"` in one place and the target repo in another, and `status` then
/// called the site stale forever. `std::path::absolute` is the right tool
/// rather than `canonicalize` — it touches no filesystem, so a book that has
/// not been rendered yet still has a name, and a book reached through a symlink
/// keeps the name the reader typed.
#[must_use]
pub fn book_name(book_root: &Path, fallback: &str) -> String {
    let absolute = std::path::absolute(book_root).ok();
    absolute.as_deref().and_then(Path::file_name).map_or_else(
        || fallback.to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The repo-level boilerplate a book never shows: the configured `template/`
/// directory, read from disk.
///
/// Public because four commands need the same answer. `build` commits it,
/// `verify` checks against it, and `status` and `push` compare against it — and
/// a second reading of the same directory is a second chance to disagree.
///
/// An unconfigured template is an empty set, not an error: a book without
/// scaffolding is a normal book.
///
/// # Errors
///
/// If the configured directory exists but cannot be read.
pub fn scaffolding(
    config: &BookConfig,
    book_root: &Path,
    repo_name: &str,
) -> std::io::Result<Blobs> {
    let Some(dir) = config
        .repos
        .get(repo_name)
        .and_then(|r| r.template.as_ref())
    else {
        return Ok(Blobs::new());
    };
    read_dir_recursive(&book_root.join(dir))
}

/// The files a replay leaves in the working tree: the scaffolding, the final
/// step's tree overlaid on it, and the generated `STEPS.md`.
///
/// `STEPS.md` lists every step, later ones included, so it can only live in the
/// final tree — writing it at every step would make each commit's tree depend
/// on commits that do not exist yet.
#[must_use]
pub fn final_blobs(
    plan: &RepoPlan,
    book_name: &str,
    site: Option<&str>,
    scaffolding: &Blobs,
) -> Blobs {
    let Some(last) = plan.steps.last() else {
        return scaffolding.clone();
    };
    let mut blobs = scaffolding.clone();
    blobs.extend(blobs_of(&last.tree));
    let md = trailers::steps_md(plan, book_name, site);
    blobs.insert("STEPS.md".to_string(), (md.into_bytes(), false));
    blobs
}

/// Every tag a replay of `plan` creates: one per step, plus one per chapter.
///
/// `status` compares a built repository against this list, so it lives beside
/// the code that creates the tags rather than being re-derived elsewhere.
#[must_use]
pub fn expected_tags(plan: &RepoPlan) -> Vec<String> {
    let mut tags: Vec<String> = plan.steps.iter().map(PlannedStep::tag).collect();
    let mut stems: Vec<String> = Vec::new();
    for step in &plan.steps {
        let stem = chapter_stem(&step.anchor.chapter);
        if stems.last() != Some(&stem) {
            stems.push(stem);
        }
    }
    tags.extend(stems.into_iter().map(|s| format!("{s}-end")));
    tags
}

/// The last commit of each chapter, in chapter order — the anchor for the
/// `<chapter>-end` tags.
fn chapter_ends(
    plan: &RepoPlan,
    repo: &gix::Repository,
    _head: Option<ObjectId>,
) -> Result<Vec<(String, usize, ObjectId)>, ReplayError> {
    let mut out: Vec<(String, usize, ObjectId)> = Vec::new();
    for step in &plan.steps {
        let stem = chapter_stem(&step.anchor.chapter);
        let tag = step.tag();
        let id = repo
            .find_reference(&format!("refs/tags/{tag}"))
            .map_err(git)?
            .into_fully_peeled_id()
            .map_err(git)?
            .detach();
        match out.last_mut() {
            Some((prev, seq, oid)) if *prev == stem => {
                *seq = step.seq;
                *oid = id;
            }
            _ => out.push((stem, step.seq, id)),
        }
    }
    Ok(out)
}

/// `src/ch01-a-repo.md` → `ch01-a-repo`. Public because `status` must expect
/// exactly the tag names `replay` creates; two copies of this rule would be two
/// answers to the same question.
#[must_use]
pub fn chapter_stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod replay_tests {
    use super::*;

    #[test]
    fn chapter_stem__strips_directory_and_extension() {
        assert_eq!(chapter_stem("src/ch01-a-repo.md"), "ch01-a-repo");
        assert_eq!(chapter_stem("ch06-ci.md"), "ch06-ci");
    }

    #[test]
    fn book_name__does_not_depend_on_how_the_root_is_addressed() {
        // `--book` defaults to `.`, so running `bower` from inside the book is
        // the *normal* invocation, not an edge case. `.` has no last path
        // component, so a name taken straight from `file_name` fell back — and
        // the five call sites do not pass the same fallback. `STEPS.md` and
        // `.bower-site` then named different books and `status` reported a
        // site that was stale forever.
        let cwd = std::env::current_dir().unwrap();
        let expected = cwd.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(book_name(Path::new("."), "fallback"), expected);
        assert_eq!(book_name(Path::new("./"), "fallback"), expected);
        assert_eq!(book_name(&cwd, "fallback"), expected);
    }

    #[test]
    fn book_name__falls_back_when_the_path_names_no_directory() {
        // The fallback still has a job: the root directory is not a book.
        assert_eq!(book_name(Path::new("/"), "fallback"), "fallback");
    }
}
