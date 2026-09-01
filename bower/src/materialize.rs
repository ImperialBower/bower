//! Turning a kernel `TreeState` into files — on disk, or on the way into a git
//! object database.
//!
//! Two callers need the same rules: `replay` writes trees into git and the
//! final step into a working tree; `verify` writes every step into a scratch
//! directory to run a compiler against it. The rules that matter — what counts
//! as executable, how a directory is emptied first — live here so the two
//! cannot drift.

use std::collections::BTreeMap;
use std::path::Path;

use bower_core::prelude::{FileBody, TreeState};

/// One file destined for disk or a tree: its bytes, and whether it is
/// executable.
pub type Blobs = BTreeMap<String, (Vec<u8>, bool)>;

/// A tree's files as bytes, with the executable bit inferred from a shebang.
///
/// The kernel does not model file modes — `FileBody` is text or bytes
/// (`bower-core/src/tree.rs:15`) — so the rule has to live outside it. A file
/// that opens `#!` is executable; nothing else is. That covers
/// `bin/security-scan`, which the generated `Makefile` invokes directly.
#[must_use]
pub fn blobs_of(tree: &TreeState) -> Blobs {
    tree.0
        .iter()
        .map(|(path, body)| {
            let (bytes, exec) = match body {
                FileBody::Text(t) => (t.clone().into_bytes(), t.starts_with("#!")),
                FileBody::Binary(b) => (b.clone(), false),
            };
            (path.clone(), (bytes, exec))
        })
        .collect()
}

/// Every file under `root`, recursively, keyed by its path relative to `root`.
///
/// # Errors
///
/// Any read failure, with the offending path attached by the caller.
pub fn read_dir_recursive(root: &Path) -> std::io::Result<Blobs> {
    let mut out = Blobs::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let bytes = std::fs::read(&path)?;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let exec = bytes.starts_with(b"#!");
            out.insert(rel, (bytes, exec));
        }
    }
    Ok(out)
}

/// Write `blobs` into `dir`, leaving anything already there alone.
///
/// This is the form `replay` needs: the target directory holds a `.git` that
/// must survive.
///
/// # Errors
///
/// Any filesystem failure while creating directories or writing files.
pub fn write_files(dir: &Path, blobs: &Blobs) -> std::io::Result<()> {
    for (path, (bytes, exec)) in blobs {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, bytes)?;
        set_executable(&target, *exec)?;
    }
    Ok(())
}

/// Write `blobs` into `dir`, removing whatever was there first.
///
/// Emptying the directory is the point, not a convenience: a verifier that
/// leaves step N's files behind would let a deleted file appear to survive into
/// step N+1, and the book's `op="delete"` steps would silently stop meaning
/// anything. Never call this on a directory holding a `.git` you want to keep —
/// that is what [`write_files`] is for.
///
/// # Errors
///
/// Any filesystem failure while clearing, creating, or writing.
pub fn write_tree_to_disk(dir: &Path, blobs: &Blobs) -> std::io::Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;
    write_files(dir, blobs)
}

/// Set or clear a file's executable bit.
///
/// # Errors
///
/// Any failure to change the file's permissions.
#[cfg(unix)]
pub fn set_executable(path: &Path, exec: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if exec { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

/// No-op off unix, where there is no executable bit to set.
///
/// # Errors
///
/// Never; the signature matches the unix version.
#[cfg(not(unix))]
pub fn set_executable(_path: &Path, _exec: bool) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod materialize_tests {
    use super::*;

    fn scratch(case: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bower-materialize-{case}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn tree(pairs: &[(&str, &str)]) -> TreeState {
        TreeState(
            pairs
                .iter()
                .map(|(p, t)| ((*p).to_string(), FileBody::Text((*t).to_string())))
                .collect(),
        )
    }

    #[test]
    fn blobs_of__marks_shebang_scripts_executable() {
        let blobs = blobs_of(&tree(&[
            ("bin/scan", "#!/usr/bin/env bash\nexit 0\n"),
            ("README.md", "# hi\n"),
        ]));
        assert!(blobs["bin/scan"].1, "a shebang script must be executable");
        assert!(!blobs["README.md"].1, "prose must not be executable");
    }

    #[test]
    fn materialize__writes_nested_paths() {
        let dir = scratch("nested");
        write_tree_to_disk(
            &dir,
            &blobs_of(&tree(&[(".github/workflows/ci.yml", "name: ci\n")])),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join(".github/workflows/ci.yml")).unwrap(),
            "name: ci\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn materialize__sets_the_executable_bit() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("exec");
        write_tree_to_disk(
            &dir,
            &blobs_of(&tree(&[
                ("bin/scan", "#!/bin/sh\nexit 0\n"),
                ("plain.txt", "hello\n"),
            ])),
        )
        .unwrap();
        let mode = |p: &str| {
            std::fs::metadata(dir.join(p)).unwrap().permissions().mode() & 0o777
        };
        assert_eq!(mode("bin/scan"), 0o755);
        assert_eq!(mode("plain.txt"), 0o644);
    }

    #[test]
    fn materialize__removes_what_was_there_before() {
        let dir = scratch("clean");
        write_tree_to_disk(&dir, &blobs_of(&tree(&[("gone.txt", "x\n")]))).unwrap();
        assert!(dir.join("gone.txt").exists());

        write_tree_to_disk(&dir, &blobs_of(&tree(&[("kept.txt", "y\n")]))).unwrap();
        assert!(dir.join("kept.txt").exists());
        assert!(
            !dir.join("gone.txt").exists(),
            "a stale file survived; `op=\"delete\"` steps would stop meaning anything"
        );
    }

    #[test]
    fn read_dir_recursive__round_trips_what_was_written() {
        let dir = scratch("roundtrip");
        let blobs = blobs_of(&tree(&[
            ("a/b/c.txt", "deep\n"),
            ("bin/run", "#!/bin/sh\n"),
        ]));
        write_tree_to_disk(&dir, &blobs).unwrap();
        assert_eq!(read_dir_recursive(&dir).unwrap(), blobs);
    }
}
