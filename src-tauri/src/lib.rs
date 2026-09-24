//! Application Tauri : expose le cœur (`ramure-core`) au front par des commandes IPC en
//! lecture seule, et surveille le dépôt pour rafraîchir le graph automatiquement.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use ramure_core::Repo;
use ramure_core::gitcli;
use ramure_core::search::SearchResult;
use ramure_core::view::{self, Details, EdgeDto, RepoSummary, Row};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
struct AppState {
    repo: Mutex<Option<Arc<Repo>>>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    /// Incrémenté à chaque ouverture : un rechargement lancé pour un dépôt précédent est ignoré.
    generation: Mutex<u64>,
}

fn current(state: &State<AppState>) -> Result<Arc<Repo>, String> {
    state.repo.lock().unwrap().clone().ok_or_else(|| "aucun dépôt ouvert".to_string())
}

#[tauri::command]
async fn open_repo(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<RepoSummary, String> {
    let repo = tauri::async_runtime::spawn_blocking(move || Repo::open(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let repo = Arc::new(repo);
    let s = view::summary(&repo);
    let generation = {
        let mut g = state.generation.lock().unwrap();
        *g += 1;
        *g
    };
    let (workdir, git_dir) = (repo.workdir.clone(), repo.git_dir.clone());
    *state.repo.lock().unwrap() = Some(repo);
    *state.watcher.lock().unwrap() = watch(app, workdir, git_dir, generation).ok();
    Ok(s)
}

#[tauri::command]
async fn rows(state: State<'_, AppState>, start: u32, end: u32) -> Result<Vec<Row>, String> {
    let repo = current(&state)?;
    Ok(view::rows(&repo, start, end))
}

#[tauri::command]
async fn edges(state: State<'_, AppState>, start: u32, end: u32) -> Result<Vec<EdgeDto>, String> {
    let repo = current(&state)?;
    Ok(view::edges(&repo, start, end))
}

#[tauri::command]
async fn search(state: State<'_, AppState>, query: String) -> Result<SearchResult, String> {
    Ok(current(&state)?.search.search(&query))
}

#[tauri::command]
async fn highlights(state: State<'_, AppState>, query: String, rows: Vec<u32>) -> Result<Vec<Vec<u32>>, String> {
    Ok(current(&state)?.search.highlights(&query, &rows))
}

#[tauri::command]
async fn find_row(state: State<'_, AppState>, sha: String) -> Result<Option<u32>, String> {
    Ok(current(&state)?.find_row(&sha))
}

#[tauri::command]
async fn commit_details(state: State<'_, AppState>, row: u32) -> Result<Details, String> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || view::details(&repo, row).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn file_diff(
    state: State<'_, AppState>,
    row: u32,
    path: String,
    old_path: Option<String>,
    untracked: bool,
) -> Result<String, String> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        view::file_diff(&repo, row, &path, old_path.as_deref(), untracked).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Clone, Copy, PartialEq)]
enum Change {
    /// Refs, HEAD, index, opération en cours : le graph peut changer.
    Refs,
    /// Fichiers de l'arbre de travail : seul le statut (WIP) change.
    Worktree,
}

fn classify(path: &Path, git_dir: &Path) -> Option<Change> {
    if let Ok(rel) = path.strip_prefix(git_dir) {
        let first = rel.components().next()?.as_os_str().to_string_lossy();
        // Les objets s'écrivent avant la mise à jour des refs ; les verrous sont transitoires.
        if first == "objects" || path.extension().is_some_and(|e| e == "lock") {
            return None;
        }
        return Some(Change::Refs);
    }
    Some(Change::Worktree)
}

/// Surveille le dépôt et prévient le front : `repo-changed` (graph rechargé) ou
/// `status-changed` (seul le statut de l'arbre de travail a changé).
fn watch(app: AppHandle, workdir: PathBuf, git_dir: PathBuf, generation: u64) -> notify::Result<notify::RecommendedWatcher> {
    let (tx, rx) = mpsc::channel::<Change>();
    let gd = git_dir.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            if matches!(ev.kind, notify::EventKind::Access(_)) {
                return;
            }
            for p in &ev.paths {
                if let Some(c) = classify(p, &gd) {
                    let _ = tx.send(c);
                }
            }
        }
    })?;
    watcher.watch(&workdir, RecursiveMode::Recursive)?;
    if !git_dir.starts_with(&workdir) {
        watcher.watch(&git_dir, RecursiveMode::Recursive)?;
    }
    std::thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            // Regroupe les rafales d'événements (un checkout en produit des centaines).
            let mut kind = first;
            while let Ok(next) = rx.recv_timeout(Duration::from_millis(200)) {
                if next == Change::Refs {
                    kind = Change::Refs;
                }
            }
            let state = app.state::<AppState>();
            if *state.generation.lock().unwrap() != generation {
                return; // un autre dépôt a été ouvert
            }
            let Some(repo) = state.repo.lock().unwrap().clone() else { return };
            if kind == Change::Worktree {
                let status = gitcli::status(&repo.workdir, &repo.git_dir).unwrap_or_default();
                if status.is_dirty() == repo.status.is_dirty() && status.operation == repo.status.operation {
                    let _ = app.emit("status-changed", status);
                    continue;
                }
            }
            match Repo::open(&repo.workdir) {
                Ok(fresh) => {
                    if *state.generation.lock().unwrap() != generation {
                        return;
                    }
                    let s = view::summary(&fresh);
                    *state.repo.lock().unwrap() = Some(Arc::new(fresh));
                    let _ = app.emit("repo-changed", s);
                }
                Err(e) => {
                    let _ = app.emit("repo-error", e.to_string());
                }
            }
        }
    });
    Ok(watcher)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_repo,
            rows,
            edges,
            search,
            highlights,
            find_row,
            commit_details,
            file_diff,
            initial_path
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de Ramure");
}

/// Dépôt passé en argument (`ramure <chemin>`), sinon le dossier courant s'il est dans un dépôt
/// git. Un argument qui ne désigne pas un dépôt n'est pas remplacé par le dossier courant.
#[tauri::command]
fn initial_path() -> Option<String> {
    let candidate = match std::env::args().skip(1).find(|a| !a.starts_with('-')) {
        Some(arg) => PathBuf::from(arg),
        None => std::env::current_dir().ok()?,
    };
    let candidate = candidate.canonicalize().ok()?;
    candidate
        .ancestors()
        .any(|a| a.join(".git").exists())
        .then(|| candidate.display().to_string())
}
