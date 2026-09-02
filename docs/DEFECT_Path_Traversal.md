# DEFECT: path traversal via book-controlled file paths

**Found** 2 September 2026, in review. **Reproduced, fixed, and regression-tested
the same day.** Severity: high — arbitrary file write outside the target
directory.

## What was wrong

A book is data. `file="…"` in a directive, and every link in `SUMMARY.md`, are
book-controlled strings that reached `Path::join` unvalidated:

- `bower/src/materialize.rs` — `write_files` did `dir.join(path)`
- `bower/src/loader.rs` — `load` did `root.join("src").join(&link)`

Two shapes escape a `join`:

| Shape | Effect |
|---|---|
| `../../x` | climbs out of the directory |
| `/etc/cron.d/x` | **`Path::join` discards its base entirely** when the argument is absolute |

## What was actually exploitable

Reproduced against the real binary before fixing anything:

| Vector | Before |
|---|---|
| `bower verify` + `file="../../X"` | **wrote outside the work directory** |
| `bower verify` + absolute `file` | **wrote to the absolute path** |
| `bower build` + `file="../../X"` | blocked — by `gix` refusing `..` as a tree filename |
| `bower build` + absolute `file` | **wrote to the absolute path** — `gix` never protected this |
| `SUMMARY.md` link `../../../etc/hosts.md` | path constructed and read attempted |

Two things the original review had inverted, and worth recording:

1. **`bower verify` was the exploitable path, not `bower build`.** `verify`
   writes each step's tree to disk with no git anywhere in the loop. The review
   named `build` and did not mention `verify`.
2. **`build` was protected by accident.** `gix` rejects `..` in a tree filename.
   That is not our guarantee, it never covered absolute paths, and it would have
   vanished the day `gix` was swapped for `git2` — which EPIC-01's corrigendum
   explicitly contemplates.

## Threat model

A book is a repository people clone in order to contribute to. CI running
`bower verify` on a pull request would have executed this. The tool's entire
purpose is processing books it did not write.

## The fix

One guard at the I/O boundary, not three scattered checks:

- `materialize::check_path` refuses empty paths, backslashes, absolute paths,
  drive prefixes, and any `..` component.
- `materialize::check_all` applies it to a whole tree.
- `write_files` and `write_tree_to_disk` call it. `write_tree_to_disk` calls it
  **before** `remove_dir_all` — a check on the far side of the wipe would
  destroy the caller's directory on its way to refusing.
- `replay::write_tree` calls it too, so `build` refuses by our rule rather than
  relying on `gix`.
- `loader::load` refuses an unsafe `SUMMARY.md` link with `LoadError::UnsafeLink`
  — refused, not skipped: a book asking to read `/etc/passwd` is not a book with
  a typo.

**`bower-core` is unchanged.** The kernel keeps producing whatever the book says;
refusing to *act* on it is the I/O layer's job. That boundary already existed and
was used as intended.

## Tests

Eleven, at two levels. Unit tests in `bower/src/materialize.rs` cover each
rejected shape, plus `write_tree_to_disk__refuses_before_clearing_the_directory`.
`bower/tests/traversal.rs` drives the real binary through all five vectors above,
including `build_refuses_a_relative_escape_by_our_own_rule`, which asserts the
error text is ours rather than `gix`'s.

## Still open

- **Symlinks are not considered.** A book cannot create one — the kernel has no
  op for it — but a `template/` directory could contain one, and
  `read_dir_recursive` would follow it. Out of scope here; recorded in
  `docs/TECHNICAL_DEBT.md`.
- **Case-insensitive filesystems.** macOS would treat `SRC/lib.rs` and
  `src/lib.rs` as one file. Not an escape, but a way for two steps to collide
  silently.
