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

/// Reject any repo path that would let a book write outside the directory it
/// was given.
///
/// A book is data, and `file="…"` in a directive is book-controlled. Two shapes
/// escape a `dir.join(path)`:
///
/// * a `..` component, which climbs out; and
/// * an **absolute** path, which is worse — `Path::join` silently *discards*
///   its base when the argument is absolute, so `/etc/cron.d/x` escapes with no
///   traversal sequence to notice.
///
/// Windows separators and drive prefixes are refused too, because a `..\` is a
/// `..` on the platform that matters and a book should not carry either.
///
/// The kernel is unchanged by this: it keeps producing whatever the book says.
/// Refusing to *act* on it is the I/O layer's job, and this is the I/O layer.
///
/// # Errors
///
/// [`std::io::ErrorKind::InvalidInput`], naming the offending path.
pub fn check_path(path: &str) -> std::io::Result<()> {
    let bad = |why: &str| {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unsafe path in book: `{path}` — {why}"),
        ))
    };

    if path.is_empty() {
        return bad("empty");
    }
    if path.contains('\\') {
        return bad("contains a backslash");
    }
    // Absolute in the POSIX sense, or a Windows drive or UNC prefix.
    if path.starts_with('/') || Path::new(path).is_absolute() {
        return bad("is absolute");
    }
    let drive = path.as_bytes();
    if drive.len() >= 2 && drive[1] == b':' && drive[0].is_ascii_alphabetic() {
        return bad("has a drive prefix");
    }
    if path.split('/').any(|c| c == "..") {
        return bad("climbs above the repository root");
    }
    Ok(())
}

/// [`check_path`] for every path in a tree.
///
/// # Errors
///
/// The first offending path, so a book author fixes one thing at a time.
pub fn check_all(blobs: &Blobs) -> std::io::Result<()> {
    for path in blobs.keys() {
        check_path(path)?;
    }
    Ok(())
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
    check_all(blobs)?;
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
    // Before the wipe, not after: a check on the far side of `remove_dir_all`
    // would destroy the caller's directory on its way to refusing.
    check_all(blobs)?;
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
    fn check_path__accepts_ordinary_nested_paths() {
        for ok in [
            "Cargo.toml",
            "src/lib.rs",
            ".github/workflows/ci.yml",
            "bin/scan",
        ] {
            assert!(check_path(ok).is_ok(), "{ok} should be allowed");
        }
    }

    #[test]
    fn check_path__rejects_parent_components() {
        for bad in ["../x", "a/../../x", "src/../../../etc/passwd", ".."] {
            assert!(check_path(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn check_path__rejects_absolute_paths() {
        // The nastier variant: `Path::join` *discards* its base when given an
        // absolute path, so this escapes with no `..` to grep for.
        for bad in ["/etc/passwd", "/tmp/x", "//server/share"] {
            assert!(check_path(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn check_path__rejects_windows_shapes() {
        for bad in ["C:\\Windows\\x", "..\\x", "a\\..\\b", "\\\\server\\share"] {
            assert!(check_path(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn write_files__refuses_to_escape_its_directory() {
        let dir = scratch("escape");
        std::fs::create_dir_all(&dir).unwrap();
        let mut blobs = Blobs::new();
        blobs.insert("../ESCAPED.txt".to_string(), (b"no".to_vec(), false));

        let err = write_files(&dir, &blobs).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            !dir.parent().unwrap().join("ESCAPED.txt").exists(),
            "the file was written despite the refusal"
        );
    }

    #[test]
    fn write_tree_to_disk__refuses_before_clearing_the_directory() {
        // Order matters: a check after the wipe would destroy the caller's
        // directory on the way to refusing.
        let dir = scratch("nowipe");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("keep.txt"), "important").unwrap();

        let mut blobs = Blobs::new();
        blobs.insert("/etc/passwd".to_string(), (b"no".to_vec(), false));

        assert!(write_tree_to_disk(&dir, &blobs).is_err());
        assert!(
            dir.join("keep.txt").exists(),
            "the directory was wiped before the path was checked"
        );
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
        let mode = |p: &str| std::fs::metadata(dir.join(p)).unwrap().permissions().mode() & 0o777;
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
