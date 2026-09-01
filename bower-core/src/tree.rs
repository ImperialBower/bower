//! The tree fold: turning blocks into complete file trees, one state per
//! step. This module owns content assembly (mdBook hidden lines, marker
//! stripping), the region engine, and every op's application semantics.

use std::collections::BTreeMap;

use crate::block::Block;
use crate::directive::Op;
use crate::source::Location;
use crate::{BowerError, Errors};

/// A file's content. Text files are stored newline-joined with a trailing
/// newline; `op="copy"` assets are bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileBody {
    Text(String),
    Binary(Vec<u8>),
}

/// The complete file tree of a repo after some step — the thing the replay
/// layer will write out and commit. `BTreeMap` keeps iteration (and thus
/// everything downstream) deterministic.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TreeState(pub BTreeMap<String, FileBody>);

impl TreeState {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn get(&self, path: &str) -> Option<&FileBody> {
        self.0.get(path)
    }

    /// Text content of a file, if present and textual.
    #[must_use]
    pub fn text(&self, path: &str) -> Option<&str> {
        match self.0.get(path) {
            Some(FileBody::Text(s)) => Some(s),
            _ => None,
        }
    }

    /// Paths in the tree, in deterministic order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// The tree as the replay layer should write it: region markers
    /// stripped unless the repo keeps them. Display markers never reach a
    /// `TreeState` at all — they are removed at assembly.
    #[must_use]
    pub fn materialized(&self, keep_region_markers: bool) -> TreeState {
        if keep_region_markers {
            return self.clone();
        }
        let mut out = BTreeMap::new();
        for (path, body) in &self.0 {
            let body = match body {
                FileBody::Binary(b) => FileBody::Binary(b.clone()),
                FileBody::Text(t) => FileBody::Text(strip_region_markers(t)),
            };
            out.insert(path.clone(), body);
        }
        TreeState(out)
    }

    /// Apply one block to the tree. `step` names the step for error
    /// context; `assets` backs `op="copy"`.
    pub fn apply_block(
        &mut self,
        block: &Block,
        step: &str,
        assets: &BTreeMap<String, Vec<u8>>,
        errors: &mut Errors,
    ) {
        match block.op {
            Op::Prose => {}
            Op::Delete => self.apply_delete(block, step, errors),
            Op::Copy => self.apply_copy(block, step, assets, errors),
            Op::Create | Op::Replace | Op::Append | Op::Region => {
                self.apply_text_op(block, step, errors);
            }
        }
    }

    fn apply_delete(&mut self, block: &Block, step: &str, errors: &mut Errors) {
        let targets: Vec<&String> = block.file.iter().chain(block.paths.iter()).collect();
        for path in targets {
            if self.0.remove(path).is_none() {
                errors.push(BowerError::FileNotCreated {
                    loc: block.loc.clone(),
                    step: step.to_string(),
                    file: path.clone(),
                });
            }
        }
    }

    fn apply_copy(
        &mut self,
        block: &Block,
        step: &str,
        assets: &BTreeMap<String, Vec<u8>>,
        errors: &mut Errors,
    ) {
        let (Some(file), Some(src)) = (&block.file, &block.src) else {
            return; // missing keys already reported at extraction
        };
        let Some(bytes) = assets.get(src) else {
            errors.push(BowerError::AssetMissing {
                loc: block.loc.clone(),
                src: src.clone(),
            });
            return;
        };
        if self.0.contains_key(file) {
            errors.push(BowerError::FileAlreadyExists {
                loc: block.loc.clone(),
                step: step.to_string(),
                file: file.clone(),
            });
            return;
        }
        self.0.insert(file.clone(), FileBody::Binary(bytes.clone()));
    }

    fn apply_text_op(&mut self, block: &Block, step: &str, errors: &mut Errors) {
        let Some(file) = &block.file else {
            return; // missing key already reported at extraction
        };
        let incoming = assemble(block);

        match block.op {
            Op::Create => {
                if self.0.contains_key(file) {
                    errors.push(BowerError::FileAlreadyExists {
                        loc: block.loc.clone(),
                        step: step.to_string(),
                        file: file.clone(),
                    });
                    return;
                }
                self.0.insert(file.clone(), FileBody::Text(join(&incoming)));
            }
            Op::Replace => {
                if !self.0.contains_key(file) {
                    errors.push(BowerError::FileNotCreated {
                        loc: block.loc.clone(),
                        step: step.to_string(),
                        file: file.clone(),
                    });
                    return;
                }
                self.0.insert(file.clone(), FileBody::Text(join(&incoming)));
            }
            Op::Append => {
                let Some(FileBody::Text(existing)) = self.0.get(file) else {
                    errors.push(BowerError::FileNotCreated {
                        loc: block.loc.clone(),
                        step: step.to_string(),
                        file: file.clone(),
                    });
                    return;
                };
                let merged = format!("{existing}{}", join(&incoming));
                self.0.insert(file.clone(), FileBody::Text(merged));
            }
            Op::Region => self.apply_region(block, step, file.clone(), &incoming, errors),
            Op::Prose | Op::Delete | Op::Copy => {}
        }
    }

    fn apply_region(
        &mut self,
        block: &Block,
        step: &str,
        file: String,
        incoming: &[String],
        errors: &mut Errors,
    ) {
        let Some(region) = &block.region else {
            return; // missing key already reported at extraction
        };
        let Some(FileBody::Text(existing)) = self.0.get(&file) else {
            errors.push(BowerError::FileNotCreated {
                loc: block.loc.clone(),
                step: step.to_string(),
                file,
            });
            return;
        };
        let lines: Vec<&str> = existing.lines().collect();
        let begin = lines
            .iter()
            .position(|l| region_marker(l) == Some((RegionMark::Begin, region.as_str())));
        let end = lines
            .iter()
            .position(|l| region_marker(l) == Some((RegionMark::End, region.as_str())));

        match (begin, end) {
            (Some(b), Some(e)) if b < e => {
                let mut out: Vec<String> = Vec::with_capacity(lines.len());
                out.extend(lines[..=b].iter().map(ToString::to_string));
                out.extend(incoming.iter().cloned());
                out.extend(lines[e..].iter().map(ToString::to_string));
                self.0.insert(file, FileBody::Text(join(&out)));
            }
            (None, None) => errors.push(BowerError::RegionMissing {
                loc: block.loc.clone(),
                step: step.to_string(),
                file,
                region: region.clone(),
            }),
            _ => errors.push(BowerError::RegionUnbalanced {
                loc: block.loc.clone(),
                step: step.to_string(),
                file,
                region: region.clone(),
            }),
        }
    }
}

/// Assemble a block's repo content: apply the mdBook hidden-line rule and
/// drop display-marker lines. Region markers pass through — they live in
/// the tree so later `op="region"` blocks can find them.
#[must_use]
pub fn assemble(block: &Block) -> Vec<String> {
    let rustish = block.content.info.split([',', ' ']).next() == Some("rust");
    let mut out = Vec::with_capacity(block.content.lines.len());
    for raw in &block.content.lines {
        let line = if rustish {
            match hidden_line(raw) {
                Hidden::No => raw.clone(),
                Hidden::Yes(content) => {
                    if block.hidden {
                        content
                    } else {
                        continue; // hidden="false": hidden lines never reach the repo
                    }
                }
            }
        } else {
            raw.clone()
        };
        if show_marker(&line).is_some() {
            continue; // display markers are book concerns, never code
        }
        out.push(line);
    }
    out
}

pub(crate) enum Hidden {
    No,
    Yes(String),
}

/// mdBook's hidden-line rule for Rust blocks: a line whose first
/// non-whitespace is `#` followed by a space (or a lone `#`) is hidden;
/// `#[`, `#!` and friends are ordinary Rust and are not.
pub(crate) fn hidden_line(raw: &str) -> Hidden {
    let indent_len = raw.len() - raw.trim_start().len();
    let (indent, t) = raw.split_at(indent_len);
    if t == "#" {
        return Hidden::Yes(String::new());
    }
    if let Some(rest) = t.strip_prefix("# ") {
        return Hidden::Yes(format!("{indent}{rest}"));
    }
    Hidden::No
}

/// Kinds of `bower:show` marker lines.
///
/// Public so that render-time consumers — the mdBook preprocessor above all —
/// use the kernel's definition of the syntax rather than reimplementing it.
/// A second copy of this grammar in a consumer is exactly the drift this crate
/// exists to prevent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShowMark {
    /// `bower:show` or `bower:show begin <name>`
    Begin(Option<String>),
    /// `bower:show end`
    End,
}

/// Recognize a display-marker line in any comment style the books will
/// meet: `//`, `#`, `--`, `;`, or bare.
#[must_use]
pub fn show_marker(line: &str) -> Option<ShowMark> {
    let body = comment_body(line)?;
    let rest = body
        .strip_prefix("bower:show")
        .or_else(|| body.strip_prefix("bf:show"))?;
    let rest = rest.trim();
    if rest == "end" {
        return Some(ShowMark::End);
    }
    if rest.is_empty() {
        return Some(ShowMark::Begin(None));
    }
    if let Some(name) = rest.strip_prefix("begin") {
        let name = name.trim();
        return Some(ShowMark::Begin(
            (!name.is_empty()).then(|| name.to_string()),
        ));
    }
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RegionMark {
    Begin,
    End,
}

/// Recognize a region-marker line: `// bower:begin <name>` / `// bower:end <name>`.
pub(crate) fn region_marker(line: &str) -> Option<(RegionMark, &str)> {
    let body = comment_body(line)?;
    let body = body
        .strip_prefix("bf:")
        .map_or_else(|| body.strip_prefix("bower:"), Some)?;
    if let Some(name) = body.strip_prefix("begin ") {
        return Some((RegionMark::Begin, name.trim()));
    }
    if let Some(name) = body.strip_prefix("end ") {
        return Some((RegionMark::End, name.trim()));
    }
    None
}

/// Strip a leading comment token and return the trimmed remainder, but
/// only for lines that then speak Bower's marker vocabulary.
fn comment_body(line: &str) -> Option<&str> {
    let t = line.trim();
    let body = t
        .strip_prefix("//")
        .or_else(|| t.strip_prefix("#"))
        .or_else(|| t.strip_prefix("--"))
        .or_else(|| t.strip_prefix(";"))
        .unwrap_or(t)
        .trim();
    (body.starts_with("bower:") || body.starts_with("bf:")).then_some(body)
}

fn strip_region_markers(text: &str) -> String {
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| region_marker(l).is_none())
        .collect();
    join(&kept)
}

/// Join lines into file text with a single trailing newline; an empty file
/// is the empty string.
fn join<S: AsRef<str>>(lines: &[S]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut s = lines
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join("\n");
    s.push('\n');
    s
}

/// A block's location, for callers assembling error context.
#[must_use]
pub fn block_loc(block: &Block) -> Location {
    block.loc.clone()
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod tree_tests {
    use super::*;
    use crate::block::BlockContent;
    use crate::source::RepoName;

    fn block(op: Op, file: &str, lines: &[&str]) -> Block {
        Block {
            loc: Location::new("ch01.md", 1),
            repo: RepoName::new("failers"),
            op,
            expect: None,
            hidden: true,
            file: Some(file.to_string()),
            paths: vec![],
            region: None,
            src: None,
            show: None,
            step: None,
            msg: None,
            after: None,
            heading: None,
            chapter_stem: "ch01".to_string(),
            content: BlockContent {
                info: "rust".to_string(),
                lines: lines.iter().map(ToString::to_string).collect(),
            },
            seq_in_book: 0,
            play: false,
        }
    }

    fn no_assets() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::new()
    }

    #[test]
    fn create__then_replace_then_append() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(Op::Create, "a.rs", &["one"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        tree.apply_block(
            &block(Op::Replace, "a.rs", &["two"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        tree.apply_block(
            &block(Op::Append, "a.rs", &["three"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(tree.text("a.rs").unwrap(), "two\nthree\n");
    }

    #[test]
    fn create__twice_is_an_error() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(Op::Create, "a.rs", &["x"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        tree.apply_block(
            &block(Op::Create, "a.rs", &["y"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        assert!(matches!(errors.0[0], BowerError::FileAlreadyExists { .. }));
    }

    #[test]
    fn modify__before_create_is_an_error() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(Op::Append, "a.rs", &["x"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        assert!(matches!(errors.0[0], BowerError::FileNotCreated { .. }));
    }

    #[test]
    fn region__replaces_between_markers_only() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(
                Op::Create,
                "a.rs",
                &[
                    "head",
                    "// bower:begin body",
                    "old",
                    "// bower:end body",
                    "tail",
                ],
            ),
            "s",
            &no_assets(),
            &mut errors,
        );
        let mut b = block(Op::Region, "a.rs", &["new", "lines"]);
        b.region = Some("body".to_string());
        tree.apply_block(&b, "s", &no_assets(), &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(
            tree.text("a.rs").unwrap(),
            "head\n// bower:begin body\nnew\nlines\n// bower:end body\ntail\n"
        );
    }

    #[test]
    fn region__is_idempotent_under_reapplication() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(Op::Create, "a.rs", &["// bower:begin b", "// bower:end b"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        let mut b = block(Op::Region, "a.rs", &["content"]);
        b.region = Some("b".to_string());
        tree.apply_block(&b, "s", &no_assets(), &mut errors);
        let once = tree.clone();
        tree.apply_block(&b, "s", &no_assets(), &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(tree, once);
    }

    #[test]
    fn region__missing_markers_is_an_error() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(Op::Create, "a.rs", &["plain"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        let mut b = block(Op::Region, "a.rs", &["x"]);
        b.region = Some("ghost".to_string());
        tree.apply_block(&b, "s", &no_assets(), &mut errors);
        assert!(matches!(errors.0[0], BowerError::RegionMissing { .. }));
    }

    #[test]
    fn assemble__hidden_lines_join_the_repo_but_attrs_do_not_trigger() {
        let b = block(
            Op::Create,
            "a.rs",
            &["# use std::fmt;", "#[derive(Debug)]", "pub struct S;", "#"],
        );
        assert_eq!(
            assemble(&b),
            vec![
                "use std::fmt;".to_string(),
                "#[derive(Debug)]".to_string(),
                "pub struct S;".to_string(),
                String::new(),
            ]
        );
    }

    #[test]
    fn assemble__hidden_false_drops_hidden_lines() {
        let mut b = block(Op::Create, "a.rs", &["# secret", "visible"]);
        b.hidden = false;
        assert_eq!(assemble(&b), vec!["visible".to_string()]);
    }

    #[test]
    fn assemble__show_markers_never_reach_the_repo() {
        let b = block(
            Op::Create,
            "a.rs",
            &["// bower:show", "kept", "// bower:show end"],
        );
        assert_eq!(assemble(&b), vec!["kept".to_string()]);
    }

    #[test]
    fn materialized__strips_region_markers_unless_kept() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(
                Op::Create,
                "a.rs",
                &["// bower:begin b", "x", "// bower:end b"],
            ),
            "s",
            &no_assets(),
            &mut errors,
        );
        assert_eq!(tree.materialized(false).text("a.rs").unwrap(), "x\n");
        assert_eq!(
            tree.materialized(true).text("a.rs").unwrap(),
            "// bower:begin b\nx\n// bower:end b\n"
        );
    }

    #[test]
    fn delete__removes_file_and_paths() {
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        tree.apply_block(
            &block(Op::Create, "a.rs", &["x"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        tree.apply_block(
            &block(Op::Create, "b.rs", &["y"]),
            "s",
            &no_assets(),
            &mut errors,
        );
        let mut d = block(Op::Delete, "a.rs", &[]);
        d.paths = vec!["b.rs".to_string()];
        tree.apply_block(&d, "s", &no_assets(), &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert!(tree.is_empty());
    }

    #[test]
    fn copy__pulls_bytes_from_assets() {
        let mut assets = BTreeMap::new();
        assets.insert("img/logo.png".to_string(), vec![1, 2, 3]);
        let mut tree = TreeState::default();
        let mut errors = Errors::default();
        let mut c = block(Op::Copy, "assets/logo.png", &[]);
        c.src = Some("img/logo.png".to_string());
        tree.apply_block(&c, "s", &assets, &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(
            tree.get("assets/logo.png"),
            Some(&FileBody::Binary(vec![1, 2, 3]))
        );
    }
}
