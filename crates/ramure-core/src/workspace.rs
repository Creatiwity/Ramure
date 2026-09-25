//! Espaces de travail : des dossiers racines (« contextes ») dans lesquels on cherche les dépôts
//! git, et pour chacun la liste des derniers dépôts ouverts.
//!
//! Le scan ne lit que la structure des dossiers et le fichier `HEAD` de chaque dépôt ; il ne
//! descend pas dans un dépôt trouvé ni dans les dossiers lourds (`node_modules`, `target`…).
//! Le store est un fichier JSON propre à Ramure (jamais dans un dépôt).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::Error;

/// Dossiers jamais explorés : dépendances, artefacts de build, caches.
const SKIPPED: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    "out",
    "Pods",
    "DerivedData",
    "__pycache__",
    "venv",
    "Library",
    "Applications",
];

#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    /// Profondeur maximale sous la racine (la racine est à 0).
    pub max_depth: usize,
    /// Nombre maximal de dépôts renvoyés (protection contre un dossier racine trop large).
    pub max_repos: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            max_depth: 6,
            max_repos: 2000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Dir,
    Repo,
}

/// Nœud de l'arbre des dépôts, élagué (seuls les chemins qui mènent à un dépôt) et compacté :
/// une suite de dossiers qui ne contiennent chacun qu'un seul sous-dossier utile devient un seul
/// nœud (`clients / acme / apps`), comme les « compact folders » de VS Code.
#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    /// Libellé affiché : un nom, ou plusieurs segments joints par « / » après compaction.
    pub name: String,
    pub path: String,
    pub kind: NodeKind,
    /// Branche courante d'un dépôt (`None` si HEAD détaché ou illisible).
    pub branch: Option<String>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub root: String,
    pub tree: Vec<TreeNode>,
    pub repos: u32,
    /// Vrai si `max_repos` a été atteint : la liste est incomplète.
    pub truncated: bool,
    pub elapsed_ms: f64,
}

pub fn scan(root: &Path, opts: ScanOptions) -> Result<ScanResult, Error> {
    let t = std::time::Instant::now();
    if !root.is_dir() {
        return Err(Error::Open(format!("{} n'est pas un dossier", root.display())));
    }
    let mut count = 0usize;
    let mut truncated = false;
    let tree = if is_repo(root) {
        // La racine est elle-même un dépôt : on l'affiche seule.
        count = 1;
        vec![repo_node(root, name_of(root))]
    } else {
        walk(root, 1, opts, &mut count, &mut truncated)
    };
    let tree = tree.into_iter().map(compact).collect();
    Ok(ScanResult {
        root: root.display().to_string(),
        tree,
        repos: count as u32,
        truncated,
        elapsed_ms: t.elapsed().as_secs_f64() * 1000.0,
    })
}

fn walk(dir: &Path, depth: usize, opts: ScanOptions, count: &mut usize, truncated: &mut bool) -> Vec<TreeNode> {
    if depth > opts.max_depth {
        return Vec::new();
    }
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    let mut entries: Vec<(String, PathBuf)> = read
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir())) // pas de lien symbolique : pas de boucle
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (!name.starts_with('.') && !SKIPPED.contains(&name.as_str())).then(|| (name, e.path()))
        })
        .collect();
    entries.sort_by_key(|(n, _)| n.to_lowercase());
    let mut out = Vec::new();
    for (name, path) in entries {
        if *count >= opts.max_repos {
            *truncated = true;
            break;
        }
        if is_repo(&path) {
            *count += 1;
            out.push(repo_node(&path, name));
        } else {
            let children = walk(&path, depth + 1, opts, count, truncated);
            if !children.is_empty() {
                out.push(TreeNode {
                    name,
                    path: path.display().to_string(),
                    kind: NodeKind::Dir,
                    branch: None,
                    children,
                });
            }
        }
    }
    out
}

/// Fusionne un dossier avec son unique sous-dossier, récursivement.
fn compact(mut node: TreeNode) -> TreeNode {
    while node.kind == NodeKind::Dir && node.children.len() == 1 && node.children[0].kind == NodeKind::Dir {
        let child = node.children.pop().unwrap();
        node = TreeNode {
            name: format!("{} / {}", node.name, child.name),
            path: child.path,
            kind: NodeKind::Dir,
            branch: None,
            children: child.children,
        };
    }
    node.children = node.children.into_iter().map(compact).collect();
    node
}

fn is_repo(path: &Path) -> bool {
    // `.git` est un dossier (dépôt classique) ou un fichier (worktree, sous-module).
    path.join(".git").exists()
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn repo_node(path: &Path, name: String) -> TreeNode {
    TreeNode {
        name,
        path: path.display().to_string(),
        kind: NodeKind::Repo,
        branch: current_branch(path),
        children: Vec::new(),
    }
}

/// Branche courante lue directement dans `HEAD` (sans lancer git : le scan doit rester rapide).
pub fn current_branch(repo: &Path) -> Option<String> {
    let dot_git = repo.join(".git");
    let git_dir = if dot_git.is_file() {
        let content = fs::read_to_string(&dot_git).ok()?;
        let p = PathBuf::from(content.trim().strip_prefix("gitdir:")?.trim());
        if p.is_absolute() { p } else { repo.join(p) }
    } else {
        dot_git
    };
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    head.trim().strip_prefix("ref: refs/heads/").map(String::from)
}

// --- Store -----------------------------------------------------------------

/// Nombre de dépôts récents mémorisés par contexte.
pub const MAX_RECENT: usize = 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentRepo {
    pub path: String,
    /// Horodatage Unix (secondes) de la dernière ouverture.
    pub opened_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub path: String,
    pub added_at: i64,
    #[serde(default)]
    pub recent: Vec<RecentRepo>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Store {
    #[serde(default = "version")]
    pub version: u32,
    /// Dossier racine actif (le contexte courant).
    pub active: Option<String>,
    #[serde(default)]
    pub roots: Vec<Root>,
    /// Dépôts ouverts hors de tout dossier racine.
    #[serde(default)]
    pub recent_outside: Vec<RecentRepo>,
}

fn version() -> u32 {
    1
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn normalize(path: &str) -> String {
    let p = PathBuf::from(path);
    let p = p.canonicalize().unwrap_or(p);
    let s = p.display().to_string();
    if s.len() > 1 {
        s.trim_end_matches(std::path::MAIN_SEPARATOR).to_string()
    } else {
        s
    }
}

fn is_inside(repo: &str, root: &str) -> bool {
    let (r, root) = (Path::new(repo), Path::new(root));
    r.starts_with(root)
}

fn push_recent(list: &mut Vec<RecentRepo>, path: &str, at: i64) {
    list.retain(|r| r.path != path);
    list.insert(
        0,
        RecentRepo {
            path: path.to_string(),
            opened_at: at,
        },
    );
    list.truncate(MAX_RECENT);
}

impl Store {
    /// Lit le store ; un fichier absent ou illisible donne un store vide (jamais d'erreur bloquante).
    pub fn load(file: &Path) -> Store {
        fs::read_to_string(file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Store {
                version: 1,
                ..Store::default()
            })
    }

    /// Écrit le store de façon atomique (fichier temporaire puis renommage).
    pub fn save(&self, file: &Path) -> Result<(), Error> {
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir).map_err(|e| Error::Git(format!("config : {e}")))?;
        }
        let tmp = file.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap()).map_err(|e| Error::Git(format!("config : {e}")))?;
        fs::rename(&tmp, file).map_err(|e| Error::Git(format!("config : {e}")))
    }

    /// Ajoute un dossier racine (ou le réactive s'il existe) et en fait le contexte actif.
    pub fn add_root(&mut self, path: &str) -> String {
        let path = normalize(path);
        if !self.roots.iter().any(|r| r.path == path) {
            // Les dépôts ouverts « hors racine » qui sont dedans rejoignent ce contexte.
            let (inside, outside): (Vec<_>, Vec<_>) = self.recent_outside.drain(..).partition(|r| is_inside(&r.path, &path));
            self.recent_outside = outside;
            self.roots.push(Root {
                path: path.clone(),
                added_at: now(),
                recent: inside,
            });
        }
        self.active = Some(path.clone());
        path
    }

    pub fn remove_root(&mut self, path: &str) {
        let path = normalize(path);
        self.roots.retain(|r| r.path != path);
        if self.active.as_deref() == Some(path.as_str()) {
            self.active = self.roots.first().map(|r| r.path.clone());
        }
    }

    pub fn set_active(&mut self, path: Option<&str>) {
        self.active = path.map(normalize).filter(|p| self.roots.iter().any(|r| &r.path == p));
    }

    /// Mémorise l'ouverture d'un dépôt dans chaque contexte qui le contient, sinon hors racine.
    pub fn record_open(&mut self, repo: &str) {
        self.record_open_at(repo, now());
    }

    pub fn record_open_at(&mut self, repo: &str, at: i64) {
        let repo = normalize(repo);
        let mut found = false;
        for root in &mut self.roots {
            if is_inside(&repo, &root.path) {
                push_recent(&mut root.recent, &repo, at);
                found = true;
            }
        }
        if !found {
            push_recent(&mut self.recent_outside, &repo, at);
        }
    }

    /// Oublie un dépôt récent (supprimé ou déplacé sur le disque).
    pub fn forget(&mut self, repo: &str) {
        for root in &mut self.roots {
            root.recent.retain(|r| r.path != repo);
        }
        self.recent_outside.retain(|r| r.path != repo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkrepo(p: &Path) {
        fs::create_dir_all(p.join(".git")).unwrap();
        fs::write(p.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    }

    #[test]
    fn scan_prunes_compacts_and_skips() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        mkrepo(&r.join("clients/acme/apps/site"));
        mkrepo(&r.join("clients/acme/apps/api"));
        mkrepo(&r.join("perso/ramure"));
        mkrepo(&r.join("perso/ramure/vendor-copy/inner")); // dans un dépôt : ignoré
        mkrepo(&r.join("tools/node_modules/pkg")); // dossier lourd : ignoré
        mkrepo(&r.join(".cache/hidden")); // dossier caché : ignoré
        fs::create_dir_all(r.join("empty/deep/nothing")).unwrap(); // sans dépôt : élagué
        fs::write(r.join("perso/wt"), "").unwrap();
        mkrepo(&r.join("zeta"));
        fs::write(r.join("zeta/.git/HEAD"), "4f2a9c1e\n").unwrap(); // HEAD détaché

        let res = scan(r, ScanOptions::default()).unwrap();
        assert_eq!(res.repos, 4);
        assert!(!res.truncated);
        let names: Vec<&str> = res.tree.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["clients / acme / apps", "perso", "zeta"]);
        let apps = &res.tree[0];
        assert_eq!(apps.kind, NodeKind::Dir);
        assert_eq!(
            apps.children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["api", "site"]
        );
        assert_eq!(apps.children[0].branch.as_deref(), Some("main"));
        // « perso » ne contient qu'un dépôt : ce n'est pas un dossier, il n'est pas fusionné.
        assert_eq!(res.tree[1].children[0].name, "ramure");
        assert_eq!(res.tree[2].kind, NodeKind::Repo);
        assert_eq!(res.tree[2].branch, None);
    }

    #[test]
    fn scan_respects_depth_and_limit() {
        let t = tempfile::tempdir().unwrap();
        mkrepo(&t.path().join("a/b/c/d/repo"));
        for i in 0..5 {
            mkrepo(&t.path().join(format!("many/r{i}")));
        }
        let shallow = scan(
            t.path(),
            ScanOptions {
                max_depth: 3,
                max_repos: 100,
            },
        )
        .unwrap();
        assert_eq!(shallow.repos, 5);
        let limited = scan(
            t.path(),
            ScanOptions {
                max_depth: 6,
                max_repos: 3,
            },
        )
        .unwrap();
        assert_eq!(limited.repos, 3);
        assert!(limited.truncated);
    }

    #[test]
    fn root_that_is_a_repo_is_listed_alone() {
        let t = tempfile::tempdir().unwrap();
        mkrepo(t.path());
        let res = scan(t.path(), ScanOptions::default()).unwrap();
        assert_eq!(res.repos, 1);
        assert_eq!(res.tree[0].kind, NodeKind::Repo);
    }

    #[test]
    fn worktree_branch_is_read_through_gitdir_file() {
        let t = tempfile::tempdir().unwrap();
        let gd = t.path().join("main-repo/.git/worktrees/wt");
        fs::create_dir_all(&gd).unwrap();
        fs::write(gd.join("HEAD"), "ref: refs/heads/feat/x\n").unwrap();
        fs::create_dir_all(t.path().join("wt")).unwrap();
        fs::write(t.path().join("wt/.git"), format!("gitdir: {}\n", gd.display())).unwrap();
        assert_eq!(current_branch(&t.path().join("wt")).as_deref(), Some("feat/x"));
    }

    #[test]
    fn store_contexts_and_recents() {
        let t = tempfile::tempdir().unwrap();
        let code = t.path().join("code");
        let other = t.path().join("other");
        fs::create_dir_all(code.join("a")).unwrap();
        fs::create_dir_all(code.join("b")).unwrap();
        fs::create_dir_all(other.join("c")).unwrap();
        let s = |p: &Path| p.canonicalize().unwrap().display().to_string();

        let mut st = Store::default();
        // Ouvert avant d'ajouter la racine : rangé hors racine, puis rattaché à l'ajout.
        st.record_open_at(&s(&code.join("a")), 100);
        assert_eq!(st.recent_outside.len(), 1);
        let root = st.add_root(&code.display().to_string());
        assert_eq!(root, s(&code));
        assert!(st.recent_outside.is_empty());
        assert_eq!(st.active.as_deref(), Some(root.as_str()));

        st.record_open_at(&s(&code.join("b")), 200);
        st.record_open_at(&s(&code.join("a")), 300); // réouverture : remonte en tête, sans doublon
        st.record_open_at(&s(&other.join("c")), 400);
        let recent: Vec<(&str, i64)> = st.roots[0].recent.iter().map(|r| (r.path.as_str(), r.opened_at)).collect();
        assert_eq!(recent, vec![(s(&code.join("a")).as_str(), 300), (s(&code.join("b")).as_str(), 200)]);
        assert_eq!(st.recent_outside.len(), 1);

        // Persistance aller-retour.
        let file = t.path().join("cfg/workspaces.json");
        st.save(&file).unwrap();
        assert_eq!(Store::load(&file), st);

        // Suppression de la racine active : on bascule sur la suivante (aucune ici).
        st.remove_root(&root);
        assert!(st.roots.is_empty());
        assert_eq!(st.active, None);
        // Fichier absent ou corrompu : store vide, pas d'erreur.
        fs::write(&file, "{ pas du json").unwrap();
        assert!(Store::load(&file).roots.is_empty());
    }

    #[test]
    fn recents_are_capped() {
        let mut st = Store::default();
        for i in 0..30 {
            st.record_open_at(&format!("/nowhere/repo{i}"), i);
        }
        assert_eq!(st.recent_outside.len(), MAX_RECENT);
        assert_eq!(st.recent_outside[0].path, "/nowhere/repo29");
    }
}
