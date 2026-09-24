//! Cœur de Ramure : lecture du dépôt, calcul du commit graph et recherche.
//!
//! Ce crate n'écrit jamais dans le dépôt : lecture via gix, et via le binaire `git` en lecture
//! seule (statut, stashes, détails, diffs, identité).

pub mod gitcli;
pub mod graph;
pub mod repo;
pub mod search;
pub mod view;

pub use repo::{CommitInfo, CommitKind, RefInfo, RefKind, Repo};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("impossible d'ouvrir le dépôt : {0}")]
    Open(String),
    #[error("{0}")]
    Git(String),
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
