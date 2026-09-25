//! Cœur de Ramure : lecture du dépôt, calcul du commit graph et recherche.
//!
//! Ce crate n'écrit jamais dans le dépôt : lecture via gix, et via le binaire `git` en lecture
//! seule (statut, stashes, détails, diffs, identité).

pub mod gitcli;
pub mod graph;
pub mod repo;
pub mod search;
pub mod view;
pub mod workspace;

pub use repo::{CommitInfo, CommitKind, RefInfo, RefKind, Repo};

/// Erreurs du cœur. Chacune a un **code stable** (`code()`) que le front traduit, et un détail
/// technique (message de git ou de gix, chemin…) affiché tel quel. Le texte `Display` est en
/// anglais et ne sert qu'aux journaux.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("folder not found: {0}")]
    NotFound(String),
    #[error("not a directory: {0}")]
    NotADirectory(String),
    #[error("bare repositories are not supported")]
    BareRepo,
    #[error("cannot open repository: {0}")]
    Open(String),
    #[error("git is not installed or not in PATH: {0}")]
    GitMissing(String),
    #[error("git failed: {0}")]
    Git(String),
    #[error("cannot write configuration: {0}")]
    Config(String),
    #[error("no repository is open")]
    NoRepo,
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotFound(_) => "not_found",
            Error::NotADirectory(_) => "not_a_directory",
            Error::BareRepo => "bare_repo",
            Error::Open(_) => "open",
            Error::GitMissing(_) => "git_missing",
            Error::Git(_) => "git",
            Error::Config(_) => "config",
            Error::NoRepo => "no_repo",
            Error::Internal(_) => "internal",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Error::NotFound(d)
            | Error::NotADirectory(d)
            | Error::Open(d)
            | Error::GitMissing(d)
            | Error::Git(d)
            | Error::Config(d)
            | Error::Internal(d) => d.clone(),
            Error::BareRepo | Error::NoRepo => String::new(),
        }
    }
}

/// Sérialisé en `{ "code": "not_found", "detail": "/chemin" }` pour le front.
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("Error", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("detail", &self.detail())?;
        st.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_serialize_as_code_and_detail() {
        let v = serde_json::to_value(Error::NotFound("/x".into())).unwrap();
        assert_eq!(v, serde_json::json!({ "code": "not_found", "detail": "/x" }));
        let v = serde_json::to_value(Error::NoRepo).unwrap();
        assert_eq!(v, serde_json::json!({ "code": "no_repo", "detail": "" }));
    }
}
