//! Chargement d'un dépôt : commits, refs, stashes, WIP, puis calcul du graph.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use gix::ObjectId;
use rayon::prelude::*;

use crate::Error;
use crate::gitcli::{self, Identity, WorkStatus};
use crate::graph::{self, Edge, Layout, Node};
use crate::search::SearchIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitKind {
    Commit,
    Merge,
    Stash,
    Wip,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: ObjectId,
    pub parents: Vec<u32>,
    pub time: i64,
    pub author: String,
    pub email: String,
    pub summary: String,
    pub kind: CommitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefKind {
    Local,
    Remote,
    Tag,
    Stash,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RefInfo {
    /// Nom court : `main`, `origin/main`, `v0.4.0`, `stash@{0}`.
    pub name: String,
    pub full: String,
    pub kind: RefKind,
    /// Indice du commit visé.
    #[serde(skip)]
    pub node: u32,
    /// Ligne du commit visé dans le graph.
    pub row: u32,
    /// Branche locale checkoutée.
    pub head: bool,
    /// Branche locale dont une distante de même nom pointe sur le même commit.
    pub synced_remote: Option<String>,
    /// Pour une distante : masquée dans le graph car fusionnée avec sa locale.
    pub merged_into_local: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HeadInfo {
    /// Branche courante, `None` si HEAD détaché.
    pub branch: Option<String>,
    pub row: Option<u32>,
    pub detached: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Timings {
    pub refs_ms: f64,
    pub walk_ms: f64,
    pub decode_ms: f64,
    pub layout_ms: f64,
    pub index_ms: f64,
    pub status_ms: f64,
    pub total_ms: f64,
}

/// Dépôt chargé en mémoire.
pub struct Repo {
    pub workdir: PathBuf,
    pub git_dir: PathBuf,
    pub name: String,
    pub commits: Vec<CommitInfo>,
    pub refs: Vec<RefInfo>,
    pub layout: Layout,
    /// Couleur (0..8) de chaque chaîne.
    pub chain_color: Vec<u8>,
    /// Chaînes des troncs, par rang.
    pub trunk_chains: Vec<u32>,
    pub trunk_names: Vec<String>,
    pub head: HeadInfo,
    pub head_chain: Option<u32>,
    pub status: WorkStatus,
    pub identity: Identity,
    pub search: SearchIndex,
    pub timings: Timings,
    /// Refs par ligne (indices dans `refs`).
    pub refs_by_row: HashMap<u32, Vec<u32>>,
}

/// Branches considérées comme tronc, par priorité.
const TRUNK_CANDIDATES: &[&[&str]] = &[&["main", "master", "trunk"], &["develop", "dev", "development"]];

impl Repo {
    pub fn open(path: impl AsRef<Path>) -> Result<Repo, Error> {
        let t0 = Instant::now();
        if !path.as_ref().exists() {
            return Err(Error::NotFound(path.as_ref().display().to_string()));
        }
        let repo = gix::discover(path.as_ref()).map_err(|e| Error::Open(e.to_string()))?;
        let workdir = repo.workdir().map(Path::to_path_buf).ok_or(Error::BareRepo)?;
        let git_dir = repo.git_dir().to_path_buf();
        let name = workdir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();

        // 1. Refs.
        struct RawRef {
            full: String,
            name: String,
            kind: RefKind,
            id: ObjectId,
        }
        let mut raw_refs: Vec<RawRef> = Vec::new();
        let platform = repo.references().map_err(|e| Error::Open(e.to_string()))?;
        for r in platform.all().map_err(|e| Error::Open(e.to_string()))?.flatten() {
            let full = r.name().as_bstr().to_string();
            let (kind, name) = if let Some(n) = full.strip_prefix("refs/heads/") {
                (RefKind::Local, n.to_string())
            } else if let Some(n) = full.strip_prefix("refs/remotes/") {
                if n.ends_with("/HEAD") {
                    continue;
                }
                (RefKind::Remote, n.to_string())
            } else if let Some(n) = full.strip_prefix("refs/tags/") {
                (RefKind::Tag, n.to_string())
            } else {
                continue; // refs/stash est traité à part, le reste (notes, replace…) ignoré
            };
            let mut r = r;
            let Ok(commit) = r.peel_to_commit() else { continue };
            raw_refs.push(RawRef {
                full,
                name,
                kind,
                id: commit.id,
            });
        }
        let mut head_ref = repo.head().map_err(|e| Error::Open(e.to_string()))?;
        let head_branch = head_ref
            .referent_name()
            .and_then(|n| n.as_bstr().to_string().strip_prefix("refs/heads/").map(String::from));
        let head_id = head_ref.peel_to_commit().ok().map(|c| c.id);
        let refs_ms = ms(t0);

        // 2. Parcours de l'historique depuis toutes les têtes.
        let t1 = Instant::now();
        let mut tips: Vec<ObjectId> = raw_refs.iter().map(|r| r.id).collect();
        tips.extend(head_id);
        tips.sort();
        tips.dedup();
        let mut ids: Vec<ObjectId> = Vec::new();
        let mut parent_ids: Vec<Vec<ObjectId>> = Vec::new();
        if !tips.is_empty() {
            let walk = repo
                .rev_walk(tips.iter().copied())
                .use_commit_graph(true)
                .all()
                .map_err(|e| Error::Open(e.to_string()))?;
            for info in walk {
                let info = info.map_err(|e| Error::Open(e.to_string()))?;
                ids.push(info.id);
                parent_ids.push(info.parent_ids.iter().copied().collect());
            }
        }
        let walk_ms = ms(t1);

        // 3. Décodage en parallèle (auteur, date, sujet).
        let t2 = Instant::now();
        let sync = repo.clone().into_sync();
        let decoded: Vec<(i64, String, String, String)> = ids
            .par_chunks(2048)
            .map_init(
                || sync.to_thread_local(),
                |repo, chunk| {
                    chunk
                        .iter()
                        .map(|id| {
                            let Ok(commit) = repo.find_commit(*id) else {
                                return (0, String::new(), String::new(), String::new());
                            };
                            let Ok(c) = commit.decode() else {
                                return (0, String::new(), String::new(), String::new());
                            };
                            let time = c.committer().map(|s| s.seconds()).unwrap_or(0);
                            let (author, email) = c
                                .author()
                                .map(|a| (a.name.to_string().trim().to_string(), a.email.to_string().trim().to_string()))
                                .unwrap_or_default();
                            let msg = c.message;
                            let first = msg
                                .split(|b| *b == b'\n')
                                .find(|l| !l.iter().all(u8::is_ascii_whitespace))
                                .unwrap_or(&[]);
                            (time, author, email, String::from_utf8_lossy(first).trim().to_string())
                        })
                        .collect::<Vec<_>>()
                },
            )
            .flatten()
            .collect();
        let index_of: HashMap<ObjectId, u32> = ids.iter().enumerate().map(|(i, id)| (*id, i as u32)).collect();
        let mut commits: Vec<CommitInfo> = ids
            .iter()
            .zip(parent_ids)
            .zip(decoded)
            .map(|((id, parents), (time, author, email, summary))| {
                let parents: Vec<u32> = parents.iter().filter_map(|p| index_of.get(p).copied()).collect();
                let kind = if parents.len() > 1 { CommitKind::Merge } else { CommitKind::Commit };
                CommitInfo {
                    id: *id,
                    parents,
                    time,
                    author,
                    email,
                    summary,
                    kind,
                }
            })
            .collect();
        let decode_ms = ms(t2);

        // 4. Stashes (premier parent seulement : les commits internes d'index sont masqués).
        let mut refs: Vec<RefInfo> = Vec::new();
        for (i, s) in gitcli::stashes(&workdir).into_iter().enumerate() {
            let Ok(id) = ObjectId::from_hex(s.id.as_bytes()) else { continue };
            let Some(base) = s
                .parents
                .first()
                .and_then(|p| ObjectId::from_hex(p.as_bytes()).ok())
                .and_then(|p| index_of.get(&p).copied())
            else {
                continue;
            };
            let node = commits.len() as u32;
            commits.push(CommitInfo {
                id,
                parents: vec![base],
                time: s.time,
                author: s.author,
                email: s.email,
                summary: s.summary,
                kind: CommitKind::Stash,
            });
            refs.push(RefInfo {
                name: format!("stash@{{{i}}}"),
                full: format!("refs/stash@{{{i}}}"),
                kind: RefKind::Stash,
                node,
                row: 0,
                head: false,
                synced_remote: None,
                merged_into_local: false,
            });
        }

        // 5. Arbre de travail : ligne WIP en tête si modifié.
        let t3 = Instant::now();
        let status = gitcli::status(&workdir, &git_dir).unwrap_or_default();
        let head_node = head_id.and_then(|id| index_of.get(&id).copied());
        if status.is_dirty()
            && let Some(h) = head_node
        {
            commits.push(CommitInfo {
                id: ObjectId::null(repo.object_hash()),
                parents: vec![h],
                time: i64::MAX,
                author: String::new(),
                email: String::new(),
                summary: String::new(),
                kind: CommitKind::Wip,
            });
        }
        let status_ms = ms(t3);

        // 6. Refs résolues en nœuds, avec fusion locale + distante au même commit.
        for r in &raw_refs {
            let Some(&node) = index_of.get(&r.id) else { continue };
            refs.push(RefInfo {
                name: r.name.clone(),
                full: r.full.clone(),
                kind: r.kind,
                node,
                row: 0,
                head: r.kind == RefKind::Local && head_branch.as_deref() == Some(r.name.as_str()),
                synced_remote: None,
                merged_into_local: false,
            });
        }
        let locals: Vec<(usize, String, u32)> = refs
            .iter()
            .enumerate()
            .filter(|(_, r)| r.kind == RefKind::Local)
            .map(|(i, r)| (i, r.name.clone(), r.node))
            .collect();
        for (li, lname, lnode) in locals {
            if let Some(ri) = refs.iter().position(|r| {
                r.kind == RefKind::Remote && r.node == lnode && r.name.split_once('/').map(|(_, b)| b) == Some(lname.as_str())
            }) {
                refs[ri].merged_into_local = true;
                let rname = refs[ri].name.clone();
                refs[li].synced_remote = Some(rname);
            }
        }

        // 7. Troncs : main/master puis develop, en local sinon en distant.
        let mut trunk_names = Vec::new();
        let mut trunk_tips = Vec::new();
        for group in TRUNK_CANDIDATES {
            let found = group
                .iter()
                .find_map(|cand| refs.iter().find(|r| r.kind == RefKind::Local && r.name == *cand))
                .or_else(|| {
                    group.iter().find_map(|cand| {
                        refs.iter()
                            .find(|r| r.kind == RefKind::Remote && r.name.split_once('/').map(|(_, b)| b) == Some(*cand))
                    })
                });
            if let Some(r) = found
                && !trunk_tips.contains(&r.node)
            {
                trunk_names.push(r.name.clone());
                trunk_tips.push(r.node);
            }
        }

        // 8. Layout.
        let t4 = Instant::now();
        let nodes: Vec<Node> = commits
            .iter()
            .map(|c| Node {
                parents: c.parents.clone(),
                time: c.time,
            })
            .collect();
        let layout = graph::layout(&nodes, &trunk_tips);
        let layout_ms = ms(t4);

        for r in &mut refs {
            r.row = layout.row_of[r.node as usize];
        }
        let mut refs_by_row: HashMap<u32, Vec<u32>> = HashMap::new();
        for (i, r) in refs.iter().enumerate() {
            refs_by_row.entry(r.row).or_default().push(i as u32);
        }

        let trunk_chains: Vec<u32> = trunk_tips
            .iter()
            .map(|&t| layout.chain[layout.row_of[t as usize] as usize])
            .collect();
        let head_row = head_node.map(|h| layout.row_of[h as usize]);
        let head_chain = head_row.map(|r| layout.chain[r as usize]);
        let chain_color = color_chains(&layout, &commits, &refs, &trunk_chains);

        // 9. Index de recherche.
        let t5 = Instant::now();
        let search = SearchIndex::build(&layout, &commits, &refs);
        let index_ms = ms(t5);

        let identity = gitcli::identity(&workdir);
        let head = HeadInfo {
            branch: head_branch,
            row: head_row,
            detached: head_ref.is_detached(),
        };
        let timings = Timings {
            refs_ms,
            walk_ms,
            decode_ms,
            layout_ms,
            index_ms,
            status_ms,
            total_ms: ms(t0),
        };
        Ok(Repo {
            workdir,
            git_dir,
            name,
            commits,
            refs,
            layout,
            chain_color,
            trunk_chains,
            trunk_names,
            head,
            head_chain,
            status,
            identity,
            search,
            timings,
            refs_by_row,
        })
    }

    pub fn len(&self) -> usize {
        self.layout.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layout.order.is_empty()
    }

    pub fn commit_at(&self, row: u32) -> &CommitInfo {
        &self.commits[self.layout.order[row as usize] as usize]
    }

    pub fn edges(&self, start: u32, end: u32) -> Vec<Edge> {
        graph::edges_in(&self.layout.edges, start, end).copied().collect()
    }

    /// Ligne d'un commit à partir d'un sha (complet ou préfixe d'au moins 4 caractères).
    pub fn find_row(&self, sha: &str) -> Option<u32> {
        let sha = sha.to_ascii_lowercase();
        if sha.len() < 4 {
            return None;
        }
        self.layout
            .order
            .iter()
            .position(|&i| {
                let c = &self.commits[i as usize];
                c.kind != CommitKind::Wip && c.id.to_hex().to_string().starts_with(&sha)
            })
            .map(|r| r as u32)
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// Couleur de chaque chaîne : 0 et 1 pour les troncs, sinon une couleur stable dérivée du
/// nom de la branche (2..8), pour qu'une branche garde sa couleur d'une session à l'autre.
fn color_chains(layout: &Layout, commits: &[CommitInfo], refs: &[RefInfo], trunk_chains: &[u32]) -> Vec<u8> {
    let n = layout.chain_tip.len();
    let mut names: Vec<Option<String>> = vec![None; n];
    for (ch, &tip) in layout.chain_tip.iter().enumerate() {
        let tip_refs: Vec<&RefInfo> = refs.iter().filter(|r| r.node == tip).collect();
        let pick = tip_refs
            .iter()
            .find(|r| r.kind == RefKind::Local)
            .map(|r| r.name.clone())
            .or_else(|| {
                tip_refs
                    .iter()
                    .find(|r| r.kind == RefKind::Remote)
                    .map(|r| r.name.split_once('/').map(|(_, b)| b.to_string()).unwrap_or_else(|| r.name.clone()))
            });
        names[ch] = pick;
    }
    // Branches mergées sans ref : nom tiré du message du merge qui a ouvert la chaîne.
    for e in &layout.edges {
        let ch = e.chain as usize;
        if names[ch].is_none() && e.via_lane != e.from_lane {
            let child = &commits[layout.order[e.from_row as usize] as usize];
            names[ch] = merged_branch_name(&child.summary);
        }
    }
    (0..n)
        .map(|ch| {
            if let Some(rank) = trunk_chains.iter().position(|&t| t as usize == ch) {
                return rank.min(1) as u8;
            }
            match &names[ch] {
                Some(name) => 2 + (fnv1a(name.as_bytes()) % 6) as u8,
                None => 2 + (ch % 6) as u8,
            }
        })
        .collect()
}

/// `Merge pull request #142 from org/fix/pwa-cache` → `fix/pwa-cache` ;
/// `Merge branch 'feature/x' into main` → `feature/x`.
pub fn merged_branch_name(summary: &str) -> Option<String> {
    if let Some(rest) = summary.strip_prefix("Merge pull request #") {
        let from = rest.split_once(" from ")?.1.trim();
        let branch = from.split_once('/').map(|(_, b)| b).unwrap_or(from);
        return Some(branch.to_string());
    }
    if let Some(rest) = summary.strip_prefix("Merge branch '") {
        return rest.split_once('\'').map(|(b, _)| b.to_string());
    }
    if let Some(rest) = summary.strip_prefix("Merge remote-tracking branch '") {
        let b = rest.split_once('\'')?.0;
        return Some(b.split_once('/').map(|(_, x)| x).unwrap_or(b).to_string());
    }
    None
}

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_branch_names() {
        assert_eq!(
            merged_branch_name("Merge pull request #142 from creatiwity/fix/pwa-cache").as_deref(),
            Some("fix/pwa-cache")
        );
        assert_eq!(
            merged_branch_name("Merge branch 'feature/onboarding'").as_deref(),
            Some("feature/onboarding")
        );
        assert_eq!(merged_branch_name("Merge branch 'a' into b").as_deref(), Some("a"));
        assert_eq!(merged_branch_name("Merge remote-tracking branch 'origin/x'").as_deref(), Some("x"));
        assert_eq!(merged_branch_name("fix: nope"), None);
    }
}
