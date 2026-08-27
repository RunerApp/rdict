//! Rdict v0.1.0 format reader and writer.
//!
//! See `rdict-spec.md` at the repository root for the normative
//! specification. This crate implements the distribution format only;
//! the YAML source compiler, CLI, and language bindings are separate
//! follow-up tasks.

// Many public APIs in private modules are not yet exercised by the
// crate's own tests or the reader/writer pipeline. They exist for the
// upcoming CLI and binding layers. Suppress dead-code warnings until
// those consumers land.
#![allow(dead_code)]

mod ast;
mod blocks;
mod container;
mod error;
mod ffi;
mod index;
mod manifest;
pub mod media;
mod model;
mod postings;
mod primitive;
mod reader;
mod strings;
mod writer;

pub use error::Error;
pub use manifest::Manifest;
pub use model::{
    Def, Definition, Entry, Ety, Example, Form, Group, MediaAsset, MediaCompression, MediaKind,
    MediaRef, Morpheme, Note, Pack, PackMetadata, Pron, Relation, Sense, TargetOccurrence,
    TextSpan, Translation,
};
pub use reader::{
    DEFAULT_EAGER_TEXT_LIMIT, LookupEntry, MorphPostings, RdictReader, ReadMode, TagPostings,
};
pub use writer::RdictWriter;

/// Compute the SHA-1 hash of media bytes.
pub fn sha1_hash(bytes: &[u8]) -> [u8; 20] {
    media::sha1_hash(bytes)
}

/// Result alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
