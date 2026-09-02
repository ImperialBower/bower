//! # bower — the replay layer
//!
//! Everything `bower-core` refuses to do: reading a book off disk, parsing
//! `bower.toml`, writing git objects, running compilers, and rewriting chapters
//! at render time. The kernel stays pure and dependency-free; every side effect
//! in the system lives in this crate.
//!
//! This is a library so that the crate's two binaries — `bower` and
//! `mdbook-bower` — can share it. Two binaries cannot share modules any other
//! way, and duplicating the configuration parser between them would be the
//! first step toward two tools that disagree about the same book.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]

pub mod config;
pub mod forge;
pub mod loader;
pub mod materialize;
pub mod publish;
pub mod push;
pub mod render;
pub mod replay;
pub mod status;
pub mod trailers;
pub mod verify;

/// mdBook's JSON protocol. Behind the `preprocessor` feature, because it is the
/// only thing here that needs `serde_json`.
#[cfg(feature = "preprocessor")]
pub mod mdbook;
