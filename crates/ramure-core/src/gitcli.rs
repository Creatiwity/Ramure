//! Appels au binaire `git` de l'utilisateur, **en lecture seule**.
//!
//! Utilisés quand gix ne couvre pas (encore) le besoin ou quand on veut exactement le
//! résultat de la CLI : statut de l'arbre de travail, stashes, détails d'un commit, diff,
//! identité et son origine.

use std::path::Path;
use std::process::Command;

use crate::Error;

fn git(repo: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .args(["-c", "core.quotepath=off", "-c", "color.ui=false", "--no-pager"])
        // Ne jamais prendre de verrou optionnel (index) : on ne fait que lire.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C");
    cmd
}

fn run(repo: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let out = git(repo).args(args).output().map_err(|e| Error::GitMissing(e.to_string()))?;
    if !out.status.success() {
        return Err(Error::Git(format!(
            "git {} : {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(out.stdout)
}

fn run_opt(repo: &Path, args: &[&str]) -> Option<String> {
    let out = git(repo).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Version de git installée, ex. `(2, 43)`.
pub fn version() -> Option<(u32, u32)> {
    let out = Command::new("git").arg("--version").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let v = s.split_whitespace().nth(2)?;
    let mut it = v.split('.').map(|x| x.parse::<u32>().unwrap_or(0));
    Some((it.next()?, it.next()?))
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct WorkStatus {
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub conflicted: u32,
    /// Opération en cours détectée (`rebase`, `merge`, `cherry-pick`, `revert`).
    pub operation: Option<String>,
}

impl WorkStatus {
    pub fn is_dirty(&self) -> bool {
        self.staged + self.unstaged + self.untracked + self.conflicted > 0
    }
}

pub fn status(repo: &Path, git_dir: &Path) -> Result<WorkStatus, Error> {
    let out = run(repo, &["status", "--porcelain=v2", "-z", "--untracked-files=normal"])?;
    let mut st = WorkStatus::default();
    let mut entries = out.split(|b| *b == 0).peekable();
    while let Some(e) = entries.next() {
        match e.first() {
            Some(b'1') | Some(b'2') => {
                let xy = &e[2..4];
                if xy[0] != b'.' {
                    st.staged += 1;
                }
                if xy[1] != b'.' {
                    st.unstaged += 1;
                }
                if e[0] == b'2' {
                    entries.next(); // chemin d'origine du renommage
                }
            }
            Some(b'u') => st.conflicted += 1,
            Some(b'?') => st.untracked += 1,
            _ => {}
        }
    }
    st.operation = if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
        Some("rebase".into())
    } else if git_dir.join("MERGE_HEAD").exists() {
        Some("merge".into())
    } else if git_dir.join("CHERRY_PICK_HEAD").exists() {
        Some("cherry-pick".into())
    } else if git_dir.join("REVERT_HEAD").exists() {
        Some("revert".into())
    } else {
        None
    };
    Ok(st)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StashEntry {
    pub id: String,
    pub parents: Vec<String>,
    pub time: i64,
    pub author: String,
    pub email: String,
    pub summary: String,
}

pub fn stashes(repo: &Path) -> Vec<StashEntry> {
    let Ok(out) = run(
        repo,
        &["log", "-g", "--format=%H%x1f%P%x1f%ct%x1f%an%x1f%ae%x1f%gs%x1e", "refs/stash"],
    ) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out)
        .split('\x1e')
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim_start_matches('\n').split('\x1f').collect();
            (f.len() == 6).then(|| StashEntry {
                id: f[0].to_string(),
                parents: f[1].split_whitespace().map(String::from).collect(),
                time: f[2].parse().unwrap_or(0),
                author: f[3].to_string(),
                email: f[4].to_string(),
                summary: f[5].to_string(),
            })
        })
        .collect()
}

/// Un arbre de travail du dépôt (`git worktree list`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Worktree {
    pub path: String,
    /// Nom affiché : dernier segment du chemin.
    pub name: String,
    /// Branche extraite (nom court), `None` si HEAD détaché.
    pub branch: Option<String>,
    pub head: Option<String>,
    /// Worktree principal (celui qui contient le dossier `.git`).
    pub main: bool,
    /// Worktree actuellement ouvert dans Ramure.
    pub current: bool,
    pub locked: bool,
    /// Dossier disparu : à nettoyer par `git worktree prune`.
    pub prunable: bool,
}

/// Worktrees du dépôt, le principal en premier. Vide si git est trop ancien ou échoue.
pub fn worktrees(repo: &Path) -> Vec<Worktree> {
    run(repo, &["worktree", "list", "--porcelain", "-z"])
        .map(|out| parse_worktrees(&String::from_utf8_lossy(&out)))
        .unwrap_or_default()
}

fn parse_worktrees(out: &str) -> Vec<Worktree> {
    let mut list = Vec::new();
    // `-z` : un champ par NUL, une ligne vide (double NUL) entre deux worktrees.
    for record in out.split("\0\0").filter(|r| !r.trim_matches('\0').is_empty()) {
        let mut wt: Option<Worktree> = None;
        for field in record.split('\0').filter(|f| !f.is_empty()) {
            let (key, value) = field.split_once(' ').unwrap_or((field, ""));
            match key {
                "worktree" => {
                    let name = Path::new(value)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    wt = Some(Worktree {
                        path: value.to_string(),
                        name,
                        branch: None,
                        head: None,
                        main: list.is_empty(),
                        current: false,
                        locked: false,
                        prunable: false,
                    });
                }
                "HEAD" => wt.iter_mut().for_each(|w| w.head = Some(value.to_string())),
                "branch" => wt
                    .iter_mut()
                    .for_each(|w| w.branch = Some(value.strip_prefix("refs/heads/").unwrap_or(value).to_string())),
                "locked" => wt.iter_mut().for_each(|w| w.locked = true),
                "prunable" => wt.iter_mut().for_each(|w| w.prunable = true),
                _ => {}
            }
        }
        // Un dépôt nu n'a pas d'arbre de travail à ouvrir.
        if let Some(w) = wt.filter(|_| !record.split('\0').any(|f| f == "bare")) {
            list.push(w);
        }
    }
    list
}

/// Des modifications non commitées dans cet arbre de travail ? (lecture seule)
pub fn is_dirty(worktree: &Path) -> Result<bool, Error> {
    run(worktree, &["status", "--porcelain", "--untracked-files=normal"]).map(|out| !out.is_empty())
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Identity {
    pub name: Option<String>,
    pub email: Option<String>,
    /// Fichier de config d'où vient l'e-mail (`git config --show-origin`).
    pub origin: Option<String>,
    pub signing_key: Option<String>,
}

pub fn identity(repo: &Path) -> Identity {
    let email_with_origin = run_opt(repo, &["config", "--show-origin", "--get", "user.email"]);
    let (origin, email) = match email_with_origin {
        Some(s) => match s.split_once('\t') {
            Some((o, e)) => (Some(o.trim_start_matches("file:").to_string()), Some(e.to_string())),
            None => (None, Some(s)),
        },
        None => (None, None),
    };
    Identity {
        name: run_opt(repo, &["config", "--get", "user.name"]),
        email,
        origin,
        signing_key: run_opt(repo, &["config", "--get", "user.signingkey"]),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileChange {
    /// `A`, `M`, `D`, `R`, `C`, `T`.
    pub status: String,
    pub path: String,
    pub old_path: Option<String>,
    pub added: Option<u32>,
    pub deleted: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitDetails {
    pub id: String,
    pub parents: Vec<String>,
    pub author: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer: String,
    pub committer_email: String,
    pub committer_time: i64,
    /// Résultat de `%G?` : `G` bonne signature, `N` aucune, etc.
    pub signature: String,
    pub message: String,
    pub files: Vec<FileChange>,
}

pub fn commit_details(repo: &Path, id: &str) -> Result<CommitDetails, Error> {
    let out = run(
        repo,
        &[
            "show",
            "-s",
            "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%G?%x1f%B",
            id,
        ],
    )?;
    let s = String::from_utf8_lossy(&out);
    let f: Vec<&str> = s.splitn(10, '\x1f').collect();
    if f.len() < 10 {
        return Err(Error::Git(format!("unexpected `git show` output for {id}")));
    }
    let parents: Vec<String> = f[1].split_whitespace().map(String::from).collect();
    let files = changed_files(repo, parents.first().map(String::as_str), id)?;
    Ok(CommitDetails {
        id: f[0].to_string(),
        parents,
        author: f[2].to_string(),
        author_email: f[3].to_string(),
        author_time: f[4].parse().unwrap_or(0),
        committer: f[5].to_string(),
        committer_email: f[6].to_string(),
        committer_time: f[7].parse().unwrap_or(0),
        signature: f[8].to_string(),
        message: f[9].trim_end().to_string(),
        files,
    })
}

/// Fichiers modifiés entre `from` (ou rien pour un commit racine) et `to`.
pub fn changed_files(repo: &Path, from: Option<&str>, to: &str) -> Result<Vec<FileChange>, Error> {
    let (ns, num) = match from {
        Some(p) => (
            run(repo, &["diff", "-M", "--name-status", "-z", p, to])?,
            run(repo, &["diff", "-M", "--numstat", "-z", p, to])?,
        ),
        None => (
            run(
                repo,
                &["diff-tree", "--root", "-r", "-M", "--no-commit-id", "--name-status", "-z", to],
            )?,
            run(repo, &["diff-tree", "--root", "-r", "-M", "--no-commit-id", "--numstat", "-z", to])?,
        ),
    };
    let mut files = parse_name_status(&ns);
    let stats = parse_numstat(&num);
    for f in &mut files {
        if let Some((a, d)) = stats.iter().find(|(p, _, _)| *p == f.path).map(|(_, a, d)| (*a, *d)) {
            f.added = a;
            f.deleted = d;
        }
    }
    Ok(files)
}

fn parse_name_status(raw: &[u8]) -> Vec<FileChange> {
    let parts: Vec<String> = raw.split(|b| *b == 0).map(|p| String::from_utf8_lossy(p).into_owned()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let st = parts[i].trim();
        if st.is_empty() {
            i += 1;
            continue;
        }
        let kind = st[..1].to_string();
        if (kind == "R" || kind == "C") && i + 2 < parts.len() {
            out.push(FileChange {
                status: kind,
                old_path: Some(parts[i + 1].clone()),
                path: parts[i + 2].clone(),
                added: None,
                deleted: None,
            });
            i += 3;
        } else if i + 1 < parts.len() {
            out.push(FileChange {
                status: kind,
                old_path: None,
                path: parts[i + 1].clone(),
                added: None,
                deleted: None,
            });
            i += 2;
        } else {
            break;
        }
    }
    out
}

fn parse_numstat(raw: &[u8]) -> Vec<(String, Option<u32>, Option<u32>)> {
    let parts: Vec<String> = raw.split(|b| *b == 0).map(|p| String::from_utf8_lossy(p).into_owned()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let rec = parts[i].trim_start_matches('\n');
        let f: Vec<&str> = rec.splitn(3, '\t').collect();
        if f.len() < 3 {
            i += 1;
            continue;
        }
        let a = f[0].parse().ok();
        let d = f[1].parse().ok();
        if f[2].is_empty() && i + 2 < parts.len() {
            // Renommage : "a\td\t\0ancien\0nouveau\0"
            out.push((parts[i + 2].clone(), a, d));
            i += 3;
        } else {
            out.push((f[2].to_string(), a, d));
            i += 1;
        }
    }
    out
}

/// Diff unifié d'un fichier entre `from` (premier parent ; arbre vide pour un commit racine)
/// et `to`.
pub fn file_diff(repo: &Path, from: Option<&str>, to: &str, path: &str, old_path: Option<&str>) -> Result<String, Error> {
    let base = match from {
        Some(p) => p.to_string(),
        None => String::from_utf8_lossy(&run(repo, &["hash-object", "-t", "tree", "/dev/null"])?)
            .trim()
            .to_string(),
    };
    let mut args = vec!["diff", "-M", "--no-ext-diff", base.as_str(), to, "--"];
    if let Some(o) = old_path {
        args.push(o);
    }
    args.push(path);
    Ok(String::from_utf8_lossy(&run(repo, &args)?).into_owned())
}

/// Changements non commités (WIP) : fichiers indexés, non indexés, non suivis.
pub fn wip_files(repo: &Path) -> Result<Vec<FileChange>, Error> {
    let out = run(repo, &["status", "--porcelain=v2", "-z", "--untracked-files=all"])?;
    let mut files = Vec::new();
    let mut entries = out.split(|b| *b == 0);
    while let Some(e) = entries.next() {
        let s = String::from_utf8_lossy(e);
        match e.first() {
            Some(b'1') => {
                let f: Vec<&str> = s.splitn(9, ' ').collect();
                let xy = f[1];
                let st = if xy.starts_with('.') { &xy[1..2] } else { &xy[0..1] };
                files.push(FileChange {
                    status: st.to_string(),
                    path: f[8].to_string(),
                    old_path: None,
                    added: None,
                    deleted: None,
                });
            }
            Some(b'2') => {
                let f: Vec<&str> = s.splitn(10, ' ').collect();
                let old = entries.next().map(|o| String::from_utf8_lossy(o).into_owned());
                files.push(FileChange {
                    status: "R".into(),
                    path: f[9].to_string(),
                    old_path: old,
                    added: None,
                    deleted: None,
                });
            }
            Some(b'u') => {
                let f: Vec<&str> = s.splitn(11, ' ').collect();
                files.push(FileChange {
                    status: "U".into(),
                    path: f[10].to_string(),
                    old_path: None,
                    added: None,
                    deleted: None,
                });
            }
            Some(b'?') => files.push(FileChange {
                status: "?".into(),
                path: s[2..].to_string(),
                old_path: None,
                added: None,
                deleted: None,
            }),
            _ => {}
        }
    }
    Ok(files)
}

/// Diff d'un fichier non commité (indexé + non indexé, par rapport à HEAD).
pub fn wip_diff(repo: &Path, path: &str, untracked: bool) -> Result<String, Error> {
    if untracked {
        // `git diff --no-index` renvoie 1 quand il y a une différence : on ne passe pas par run().
        let out = git(repo)
            .args(["diff", "--no-index", "--no-ext-diff", "--", "/dev/null", path])
            .output()
            .map_err(|e| Error::Git(e.to_string()))?;
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    Ok(String::from_utf8_lossy(&run(repo, &["diff", "--no-ext-diff", "HEAD", "--", path])?).into_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn worktrees_porcelain_z() {
        let out = "worktree /r/main\0HEAD aaa\0branch refs/heads/main\0\0\
worktree /r/wt/agent-1\0HEAD bbb\0branch refs/heads/feat/agent\0locked\0\0\
worktree /tmp/spike\0HEAD ccc\0detached\0prunable gitdir file points to non-existent location\0\0";
        let w = parse_worktrees(out);
        assert_eq!(w.len(), 3);
        assert!(w[0].main && w[0].branch.as_deref() == Some("main") && !w[0].locked);
        assert_eq!(w[1].name, "agent-1");
        assert_eq!(w[1].branch.as_deref(), Some("feat/agent"));
        assert!(w[1].locked && !w[1].main);
        assert!(w[2].branch.is_none() && w[2].prunable);
        assert_eq!(w[2].head.as_deref(), Some("ccc"));
    }

    #[test]
    fn worktrees_ignore_bare_main() {
        let out = "worktree /r/repo.git\0bare\0\0worktree /r/wt\0HEAD aaa\0branch refs/heads/x\0\0";
        let w = parse_worktrees(out);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "/r/wt");
    }

    use super::*;

    #[test]
    fn parses_name_status_with_renames() {
        let raw = b"M\0src/a.rs\0R087\0old.rs\0new.rs\0A\0b.txt\0";
        let v = parse_name_status(raw);
        assert_eq!(v.len(), 3);
        assert_eq!(v[1].status, "R");
        assert_eq!(v[1].old_path.as_deref(), Some("old.rs"));
        assert_eq!(v[1].path, "new.rs");
    }

    #[test]
    fn parses_numstat_with_renames_and_binaries() {
        let raw = b"3\t1\tsrc/a.rs\x002\t2\t\0old.rs\0new.rs\0-\t-\timg.png\0";
        let v = parse_numstat(raw);
        assert_eq!(v[0], ("src/a.rs".into(), Some(3), Some(1)));
        assert_eq!(v[1], ("new.rs".into(), Some(2), Some(2)));
        assert_eq!(v[2], ("img.png".into(), None, None));
    }
}
