//! Objets envoyés au front (IPC Tauri, export d'exemple) : une seule définition pour les deux.

use serde::Serialize;

use crate::gitcli::{self, CommitDetails, FileChange, Identity, WorkStatus};
use crate::graph::Edge;
use crate::repo::{HeadInfo, Timings};
use crate::{CommitKind, Error, RefInfo, RefKind, Repo};

#[derive(Serialize, Clone)]
pub struct RepoSummary {
    pub path: String,
    pub name: String,
    pub rows: u32,
    pub max_lanes: u16,
    pub head: HeadInfo,
    pub head_chain: Option<u32>,
    pub trunk_chains: Vec<u32>,
    pub trunk_names: Vec<String>,
    pub refs: Vec<RefInfo>,
    pub status: WorkStatus,
    pub identity: Identity,
    pub timings: Timings,
    pub git_version: Option<String>,
}

#[derive(Serialize)]
pub struct RowRef {
    pub name: String,
    pub kind: RefKind,
    pub head: bool,
    /// Distante fusionnée dans cette pastille locale (même commit, même nom).
    pub remote: Option<String>,
}

#[derive(Serialize)]
pub struct Row {
    pub row: u32,
    pub id: String,
    pub summary: String,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub lane: u16,
    pub chain: u32,
    pub color: u8,
    pub kind: CommitKind,
    pub refs: Vec<RowRef>,
}

#[derive(Serialize)]
pub struct EdgeDto {
    #[serde(flatten)]
    pub edge: Edge,
    pub color: u8,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Details {
    Commit(CommitDetails),
    Wip { files: Vec<FileChange>, status: WorkStatus },
}

pub fn summary(repo: &Repo) -> RepoSummary {
    RepoSummary {
        path: repo.workdir.display().to_string(),
        name: repo.name.clone(),
        rows: repo.len() as u32,
        max_lanes: repo.layout.max_lanes,
        head: repo.head.clone(),
        head_chain: repo.head_chain,
        trunk_chains: repo.trunk_chains.clone(),
        trunk_names: repo.trunk_names.clone(),
        refs: repo.refs.clone(),
        status: repo.status.clone(),
        identity: repo.identity.clone(),
        timings: repo.timings.clone(),
        git_version: gitcli::version().map(|(a, b)| format!("{a}.{b}")),
    }
}

pub fn rows(repo: &Repo, start: u32, end: u32) -> Vec<Row> {
    let end = end.min(repo.len() as u32);
    (start.min(end)..end)
        .map(|row| {
            let c = repo.commit_at(row);
            let chain = repo.layout.chain[row as usize];
            let refs = repo
                .refs_by_row
                .get(&row)
                .map(|ids| {
                    ids.iter()
                        .map(|&i| &repo.refs[i as usize])
                        .filter(|r| !r.merged_into_local)
                        .map(|r| RowRef {
                            name: r.name.clone(),
                            kind: r.kind,
                            head: r.head,
                            remote: r.synced_remote.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let wip = c.kind == CommitKind::Wip;
            Row {
                row,
                id: if wip { String::new() } else { c.id.to_hex().to_string() },
                summary: c.summary.clone(),
                author: c.author.clone(),
                email: c.email.clone(),
                time: if wip { 0 } else { c.time },
                lane: repo.layout.lane[row as usize],
                chain,
                color: repo.chain_color[chain as usize],
                kind: c.kind,
                refs,
            }
        })
        .collect()
}

pub fn edges(repo: &Repo, start: u32, end: u32) -> Vec<EdgeDto> {
    repo.edges(start, end)
        .into_iter()
        .map(|edge| EdgeDto {
            color: repo.chain_color[edge.chain as usize],
            edge,
        })
        .collect()
}

pub fn details(repo: &Repo, row: u32) -> Result<Details, Error> {
    let c = repo.commit_at(row);
    if c.kind == CommitKind::Wip {
        let files = gitcli::wip_files(&repo.workdir)?;
        let status = gitcli::status(&repo.workdir, &repo.git_dir).unwrap_or_default();
        return Ok(Details::Wip { files, status });
    }
    gitcli::commit_details(&repo.workdir, &c.id.to_hex().to_string()).map(Details::Commit)
}

pub fn file_diff(repo: &Repo, row: u32, path: &str, old_path: Option<&str>, untracked: bool) -> Result<String, Error> {
    let c = repo.commit_at(row);
    if c.kind == CommitKind::Wip {
        return gitcli::wip_diff(&repo.workdir, path, untracked);
    }
    let id = c.id.to_hex().to_string();
    let parent = c.parents.first().map(|&p| repo.commits[p as usize].id.to_hex().to_string());
    gitcli::file_diff(&repo.workdir, parent.as_deref(), &id, path, old_path)
}
