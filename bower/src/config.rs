//! `bower.toml` — the replay layer's configuration.
//!
//! Everything the kernel deliberately refuses to know lives here: where to push,
//! which template seeds step 0, what command verifies a step, and the epoch that
//! makes commit SHAs reproducible. Exactly one setting crosses back into the
//! kernel — [`BookConfig::catalog`] projects `keep_region_markers` into a
//! [`RepoCatalog`] and drops the rest. Adding a key to this file must never
//! require touching `bower-core`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use bower_core::prelude::{RepoCatalog, RepoName, RepoSpec};
use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The parsed `bower.toml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookConfig {
    /// Timestamp base for deterministic commits: step `n` commits at
    /// `epoch + n` minutes. Replay never reads a clock.
    pub epoch: OffsetDateTime,
    /// Public site the rendered book lives at, for `Book-Url` trailers.
    pub site: Option<String>,
    /// The book's edition, as the author declares it — `"0.1.0"`, `"2e"`,
    /// whatever means an edition to this book. Absent means the book publishes
    /// no releases, which is a normal book.
    ///
    /// Deliberately a free string and not `semver`: an edition of a book is not
    /// an edition of a library, and the tool has no business insisting.
    pub version: Option<String>,
    pub identity: Identity,
    pub repos: BTreeMap<String, RepoConfig>,
}

/// The fixed author and committer of every generated commit. Fixed, because a
/// per-machine identity would change every SHA.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

/// One target repository's settings.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepoConfig {
    /// `owner/name`; enables push. Absent means local-only.
    pub github: Option<String>,
    /// The branch that serves the rendered book — `gh-pages`, typically.
    /// Absent means this book ships no site, which is a normal book.
    pub site_branch: Option<String>,
    /// Book-relative directory holding the artifacts to attach to a release —
    /// every `.pdf` and `.epub` directly inside it. Absent means this book
    /// publishes no releases, which is a normal book.
    ///
    /// Book-relative, like `template`, and for the same reason the rendered
    /// site already lives inside the book: an artifact of a book belongs to
    /// that book, not to whatever directory someone happened to run from.
    pub assets: Option<PathBuf>,
    /// Book-relative directory applied as step 0 scaffolding.
    pub template: Option<PathBuf>,
    /// Command that must *compile* a step's tree. Distinguishing "fails to
    /// compile" from "compiles, tests fail" needs two commands, not one
    /// (spec § 6). Defaults live in the verifier, not here.
    pub check: Option<String>,
    /// Command that must *test* a step's tree.
    pub verify: Option<String>,
    /// The one setting the kernel cares about.
    pub keep_region_markers: bool,
    pub links: LinkTemplates,
}

/// Forge URL templates. Keeping these as templates is what lets a book point at
/// Codeberg or a self-hosted forge without a code change.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LinkTemplates {
    /// The exact lines shown: `https://…/blob/{tag}/{path}#L{start}-L{end}`
    pub blob: Option<String>,
    /// The whole repository at this step: `https://…/tree/{tag}`
    pub tree: Option<String>,
    /// The diff this step introduced: `https://…/commit/{tag}`
    pub commit: Option<String>,
}

/// Why a configuration could not be loaded. Every variant names the file, so a
/// reader is never left guessing which `bower.toml` is wrong.
#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    Epoch {
        value: String,
        reason: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(f, "cannot parse {}: {source}", path.display())
            }
            Self::Epoch { value, reason } => {
                write!(f, "book epoch `{value}` is not a valid instant: {reason}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl BookConfig {
    /// Read and parse `<book_root>/bower.toml`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if the file cannot be read, is not valid TOML,
    /// carries an unknown key, or declares an epoch that is not an instant.
    pub fn load(book_root: &Path) -> Result<Self, ConfigError> {
        let path = book_root.join("bower.toml");
        let text = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path.clone(),
            source,
        })?;
        Self::parse(&text).map_err(|e| match e {
            ConfigError::Parse { source, .. } => ConfigError::Parse { path, source },
            other => other,
        })
    }

    /// Parse configuration text. Split out from [`Self::load`] so the rules can
    /// be tested without touching a filesystem.
    ///
    /// # Errors
    ///
    /// As [`Self::load`], minus the read failure.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let wire: Wire = toml::from_str(text).map_err(|source| ConfigError::Parse {
            path: PathBuf::from("bower.toml"),
            source,
        })?;

        let raw = wire.book.epoch.to_string();
        let epoch = OffsetDateTime::parse(&raw, &Rfc3339).map_err(|e| ConfigError::Epoch {
            value: raw,
            reason: e.to_string(),
        })?;

        Ok(Self {
            epoch,
            site: wire.book.site,
            version: wire.book.version,
            identity: Identity {
                name: wire.identity.name,
                email: wire.identity.email,
            },
            repos: wire
                .repos
                .into_iter()
                .map(|(name, r)| {
                    (
                        name,
                        RepoConfig {
                            github: r.github,
                            site_branch: r.site_branch,
                            assets: r.assets,
                            template: r.template,
                            check: r.check,
                            verify: r.verify,
                            keep_region_markers: r.keep_region_markers,
                            links: LinkTemplates {
                                blob: r.links.blob,
                                tree: r.links.tree,
                                commit: r.links.commit,
                            },
                        },
                    )
                })
                .collect(),
        })
    }

    /// The narrow projection the kernel accepts: repo names plus the single
    /// setting that changes pure computation. Everything else stays here.
    #[must_use]
    pub fn catalog(&self) -> RepoCatalog {
        RepoCatalog(
            self.repos
                .iter()
                .map(|(name, cfg)| {
                    (
                        RepoName::new(name),
                        RepoSpec {
                            keep_region_markers: cfg.keep_region_markers,
                        },
                    )
                })
                .collect(),
        )
    }
}

/// The wire form. `deny_unknown_fields` throughout: a typo in `bower.toml` is a
/// loud error at load, not a silently ignored setting discovered at push time.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    book: WireBook,
    identity: WireIdentity,
    #[serde(default)]
    repos: BTreeMap<String, WireRepo>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBook {
    epoch: toml::value::Datetime,
    site: Option<String>,
    version: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireIdentity {
    name: String,
    email: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRepo {
    github: Option<String>,
    site_branch: Option<String>,
    assets: Option<PathBuf>,
    template: Option<PathBuf>,
    check: Option<String>,
    verify: Option<String>,
    #[serde(default)]
    keep_region_markers: bool,
    #[serde(default)]
    links: WireLinks,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLinks {
    blob: Option<String>,
    tree: Option<String>,
    commit: Option<String>,
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod config_tests {
    use super::*;

    fn sample_book_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("books")
            .join("hello-playbook")
    }

    #[test]
    fn config__loads_the_sample_book() {
        let cfg = BookConfig::load(&sample_book_root()).unwrap();

        assert_eq!(cfg.epoch.to_string(), "2026-09-01 0:00:00.0 +00:00:00");
        assert_eq!(
            cfg.site.as_deref(),
            Some("https://abstecker.github.io/hello-playbook")
        );
        assert_eq!(cfg.identity.name, "ImperialBower Bower");
        assert_eq!(cfg.identity.email, "bower@imperialbower.example");

        let repo = cfg.repos.get("hello-playbook").unwrap();
        assert_eq!(repo.template, Some(PathBuf::from("template")));
        assert!(!repo.keep_region_markers);
        // The sample book publishes to a real remote as of 2 September 2026.
        assert_eq!(repo.github.as_deref(), Some("abstecker/hello-playbook"));
        assert_eq!(repo.check.as_deref(), Some("cargo check"));
        // Assert the value, not its absence: a test that pins what a live
        // fixture does *not* declare breaks the day someone declares it. This
        // one broke twice in one evening before the lesson stuck.
        assert_eq!(repo.site_branch.as_deref(), Some("gh-pages"));
        assert_eq!(repo.verify.as_deref(), Some("cargo test"));
        assert!(repo.links.blob.as_ref().unwrap().contains("{tag}"));
        assert!(repo.links.tree.as_ref().unwrap().contains("{tag}"));
        assert!(repo.links.commit.as_ref().unwrap().contains("{tag}"));
    }

    #[test]
    fn config__missing_epoch_is_an_error() {
        let text = concat!(
            "[book]\n",
            "site = \"https://example.invalid\"\n\n",
            "[identity]\n",
            "name = \"N\"\n",
            "email = \"e@example.invalid\"\n",
        );
        let err = BookConfig::parse(text).unwrap_err();
        assert!(
            matches!(err, ConfigError::Parse { .. }),
            "expected a parse error, got {err:?}"
        );
        assert!(err.to_string().contains("epoch"), "{err}");
    }

    #[test]
    fn config__unknown_key_is_an_error() {
        let text = concat!(
            "[book]\n",
            "epoch = 2026-09-01T00:00:00Z\n",
            "sight = \"typo\"\n\n",
            "[identity]\n",
            "name = \"N\"\n",
            "email = \"e@example.invalid\"\n",
        );
        let err = BookConfig::parse(text).unwrap_err();
        assert!(err.to_string().contains("sight"), "{err}");
    }

    #[test]
    fn config__catalog_carries_only_kernel_settings() {
        let cfg = BookConfig::load(&sample_book_root()).unwrap();
        let catalog = cfg.catalog();

        assert!(catalog.contains("hello-playbook"));
        assert_eq!(catalog.0.len(), 1);
        // `RepoSpec` has exactly one field, so this equality is the proof that
        // nothing else — github, site_branch, template, check, verify, links —
        // crossed the boundary.
        assert_eq!(
            catalog.spec(&RepoName::new("hello-playbook")),
            RepoSpec {
                keep_region_markers: false
            }
        );
    }
}
