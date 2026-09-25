//! Tests sur de vrais dépôts créés avec la CLI git.

use std::path::Path;
use std::process::Command;

use ramure_core::{CommitKind, RefKind, Repo};

fn git(dir: &Path, args: &[&str]) {
    let st = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Léa M.")
        .env("GIT_AUTHOR_EMAIL", "lea@example.com")
        .env("GIT_COMMITTER_NAME", "Léa M.")
        .env("GIT_COMMITTER_EMAIL", "lea@example.com")
        .status()
        .unwrap();
    assert!(st.success(), "git {args:?}");
}

/// Commit horodaté (dates croissantes pour un ordre déterministe).
fn commit(dir: &Path, n: u32, msg: &str) {
    std::fs::write(dir.join(format!("f{n}.txt")), format!("{n}\n")).unwrap();
    git(dir, &["add", "."]);
    let date = format!("2026-09-{:02}T10:00:00", n);
    let st = Command::new("git")
        .current_dir(dir)
        .args(["commit", "-q", "-m", msg])
        .env("GIT_AUTHOR_NAME", "Léa M.")
        .env("GIT_AUTHOR_EMAIL", "lea@example.com")
        .env("GIT_COMMITTER_NAME", "Léa M.")
        .env("GIT_COMMITTER_EMAIL", "lea@example.com")
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .status()
        .unwrap();
    assert!(st.success());
}

/// main : c1 - c2 - c4 - M(c4, c5) ; feature/pwa : c2 - c3 - c5 (mergée) ;
/// feature/scoring : M - c7 (HEAD, modifié) ; tag v0.1.0 sur c2 ; stash ; origin/main = M.
fn scenario() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    git(d, &["init", "-q", "-b", "main"]);
    git(d, &["config", "user.name", "Léa M."]);
    git(d, &["config", "user.email", "lea@example.com"]);
    commit(d, 1, "chore: initialise le projet");
    commit(d, 2, "feat(tasks): vue une tâche à la fois");
    git(d, &["tag", "v0.1.0"]);
    git(d, &["switch", "-q", "-c", "feature/pwa"]);
    commit(d, 3, "fix(pwa): invalide le cache");
    git(d, &["switch", "-q", "main"]);
    commit(d, 4, "chore(deps): passe à nuxt 4");
    git(d, &["switch", "-q", "feature/pwa"]);
    commit(d, 5, "test(pwa): couvre le service worker");
    git(d, &["switch", "-q", "main"]);
    let st = Command::new("git")
        .current_dir(d)
        .args(["merge", "-q", "--no-ff", "-m", "Merge branch 'feature/pwa'", "feature/pwa"])
        .env("GIT_AUTHOR_DATE", "2026-09-06T10:00:00")
        .env("GIT_COMMITTER_DATE", "2026-09-06T10:00:00")
        .status()
        .unwrap();
    assert!(st.success());
    git(d, &["branch", "-q", "-D", "feature/pwa"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(d, &["switch", "-q", "-c", "feature/scoring"]);
    commit(d, 7, "feat(scoring): pondère les tâches en retard");
    std::fs::write(d.join("f7.txt"), "modifié\n").unwrap();
    git(d, &["stash", "-q"]);
    std::fs::write(d.join("f1.txt"), "wip\n").unwrap();
    t
}

#[test]
fn loads_scenario_with_expected_rows_and_refs() {
    let t = scenario();
    let r = Repo::open(t.path()).unwrap();
    // 7 commits + 1 stash + 1 WIP
    assert_eq!(r.len(), 9);
    assert_eq!(r.commit_at(0).kind, CommitKind::Wip);
    assert_eq!(r.head.branch.as_deref(), Some("feature/scoring"));
    assert_eq!(r.trunk_names, vec!["main"]);

    // main reste en lane 0 sur toute sa chaîne de premiers parents.
    for row in 0..r.len() as u32 {
        let c = r.commit_at(row);
        if [
            "chore: initialise le projet",
            "feat(tasks): vue une tâche à la fois",
            "chore(deps): passe à nuxt 4",
            "Merge branch 'feature/pwa'",
        ]
        .contains(&c.summary.as_str())
        {
            assert_eq!(r.layout.lane[row as usize], 0, "{}", c.summary);
        }
    }

    // main et origin/main au même commit : fusionnées.
    let main = r.refs.iter().find(|x| x.kind == RefKind::Local && x.name == "main").unwrap();
    assert_eq!(main.synced_remote.as_deref(), Some("origin/main"));
    assert!(r.refs.iter().any(|x| x.kind == RefKind::Tag && x.name == "v0.1.0"));
    assert!(r.refs.iter().any(|x| x.kind == RefKind::Stash && x.name == "stash@{0}"));
    assert!(r.refs.iter().find(|x| x.name == "feature/scoring").unwrap().head);

    // Le merge est bien typé et la branche mergée a la même couleur que son nom le laisse prévoir.
    let merge_row = (0..r.len() as u32).find(|&i| r.commit_at(i).kind == CommitKind::Merge).unwrap();
    assert!(
        r.edges(merge_row, merge_row + 1)
            .iter()
            .any(|e| e.from_row == merge_row && e.via_lane != e.from_lane)
    );

    assert!(r.status.is_dirty());
    assert_eq!(r.identity.email.as_deref(), Some("lea@example.com"));
}

#[test]
fn search_finds_fuzzy_subject_sha_and_operators() {
    let t = scenario();
    let r = Repo::open(t.path()).unwrap();
    // Le commit et le stash créé dessus (« WIP on feature/scoring: … ») correspondent.
    let res = r.search.search("pndr retard");
    assert_eq!(res.total, 2);
    let hit = *res.rows.iter().find(|&&row| r.commit_at(row).kind == CommitKind::Commit).unwrap();
    assert_eq!(r.commit_at(hit).summary, "feat(scoring): pondère les tâches en retard");
    let hl = r.search.highlights("pndr retard", &[hit]);
    assert_eq!(hl[0].len(), "pndrretard".len());

    let res = r.search.search("pwa");
    assert_eq!(res.total, 3, "2 commits + le merge");

    let sha = r.commit_at(res.rows[0]).id.to_hex().to_string();
    let by_sha = r.search.search(&sha[..7]);
    assert_eq!(by_sha.total, 1);

    assert_eq!(r.search.search("author:léa pwa").total, res.total);
    assert_eq!(r.search.search("author:personne pwa").total, 0);
    assert!(r.search.search("ref:v0.1").total == 1);
}

#[test]
fn details_and_diffs_via_git_cli() {
    let t = scenario();
    let r = Repo::open(t.path()).unwrap();
    let row = (0..r.len() as u32)
        .find(|&i| r.commit_at(i).summary.starts_with("feat(scoring)"))
        .unwrap();
    let id = r.commit_at(row).id.to_hex().to_string();
    let d = ramure_core::gitcli::commit_details(&r.workdir, &id).unwrap();
    assert_eq!(d.files.len(), 1);
    assert_eq!(d.files[0].path, "f7.txt");
    assert_eq!(d.files[0].added, Some(1));
    let diff = ramure_core::gitcli::file_diff(&r.workdir, d.parents.first().map(String::as_str), &id, "f7.txt", None).unwrap();
    assert!(diff.contains("+7"));

    // Commit racine : diff contre l'arbre vide.
    let root = (0..r.len() as u32)
        .find(|&i| r.commit_at(i).summary.starts_with("chore: init"))
        .unwrap();
    let rid = r.commit_at(root).id.to_hex().to_string();
    let rd = ramure_core::gitcli::commit_details(&r.workdir, &rid).unwrap();
    assert!(rd.parents.is_empty());
    assert_eq!(rd.files[0].status, "A");
    assert!(
        ramure_core::gitcli::file_diff(&r.workdir, None, &rid, "f1.txt", None)
            .unwrap()
            .contains("+1")
    );

    let wip = ramure_core::gitcli::wip_files(&r.workdir).unwrap();
    assert_eq!(wip.len(), 1);
    assert_eq!(wip[0].path, "f1.txt");
}

#[test]
fn repository_is_never_modified() {
    let t = scenario();
    let snapshot = |p: &Path| {
        let out = Command::new("git")
            .current_dir(p)
            .args(["for-each-ref", "--format=%(refname) %(objectname)"])
            .output()
            .unwrap()
            .stdout;
        let idx = std::fs::read(p.join(".git/index")).unwrap();
        (out, idx)
    };
    let before = snapshot(t.path());
    let r = Repo::open(t.path()).unwrap();
    let _ = r.search.search("pwa");
    let _ = ramure_core::gitcli::wip_files(&r.workdir);
    assert_eq!(before, snapshot(t.path()));
}

#[test]
fn worktrees_listed_and_branches_checked_out_elsewhere_marked() {
    let t = tempfile::tempdir().unwrap();
    let main = t.path().join("demo");
    std::fs::create_dir(&main).unwrap();
    git(&main, &["init", "-q", "-b", "main"]);
    commit(&main, 1, "chore: initialise");
    commit(&main, 2, "feat: deux");
    git(&main, &["worktree", "add", "-q", "../demo-agent", "-b", "feat/agent"]);
    git(&main, &["worktree", "add", "-q", "--lock", "../demo-hot", "-b", "hotfix"]);
    std::fs::write(t.path().join("demo-agent/f1.txt"), "modifié\n").unwrap();

    // Depuis le worktree principal.
    let r = Repo::open(&main).unwrap();
    assert_eq!(r.worktrees.len(), 3);
    assert!(r.worktrees[0].main && r.worktrees[0].current);
    let agent = r.worktrees.iter().find(|w| w.name == "demo-agent").unwrap();
    assert_eq!(agent.branch.as_deref(), Some("feat/agent"));
    assert!(!agent.current && !agent.locked);
    assert!(r.worktrees.iter().find(|w| w.name == "demo-hot").unwrap().locked);
    let local = |r: &Repo, n: &str| {
        r.refs
            .iter()
            .find(|x| x.kind == RefKind::Local && x.name == n)
            .unwrap()
            .worktree
            .clone()
    };
    assert!(local(&r, "feat/agent").unwrap().ends_with("demo-agent"));
    assert!(local(&r, "hotfix").unwrap().ends_with("demo-hot"));
    assert_eq!(local(&r, "main"), None, "la branche du worktree ouvert n'est pas « ailleurs »");
    assert!(ramure_core::gitcli::is_dirty(Path::new(&agent.path)).unwrap());
    assert!(!ramure_core::gitcli::is_dirty(&main).unwrap());

    // Depuis un worktree lié : main est extraite ailleurs, le dossier git commun est connu.
    let w = Repo::open(t.path().join("demo-agent")).unwrap();
    assert!(w.worktrees.iter().find(|x| x.name == "demo-agent").unwrap().current);
    assert!(local(&w, "main").unwrap().ends_with("demo"));
    assert_eq!(local(&w, "feat/agent"), None);
    // Chemins déjà normalisés (le watcher les compare aux chemins des événements).
    assert_eq!(w.common_dir, main.join(".git").canonicalize().unwrap());
    assert!(w.git_dir.starts_with(&w.common_dir) && w.git_dir != w.common_dir);
    assert_eq!(r.git_dir, r.common_dir);
    assert_eq!(w.head.branch.as_deref(), Some("feat/agent"));
}
