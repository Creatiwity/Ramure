//! Application Tauri : expose le cœur (`ramure-core`) au front par des commandes IPC en
//! lecture seule, et surveille le dépôt pour rafraîchir le graph automatiquement.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use ramure_core::gitcli;
use ramure_core::search::SearchResult;
use ramure_core::view::{self, Details, EdgeDto, RepoSummary, Row};
use ramure_core::workspace::{self, ScanOptions, ScanResult, Store};
use ramure_core::{Error, Repo};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
struct AppState {
    repo: Mutex<Option<Arc<Repo>>>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    /// Incrémenté à chaque ouverture : un rechargement lancé pour un dépôt précédent est ignoré.
    generation: Mutex<u64>,
    /// Espaces de travail (dossiers racines et récents), persistés dans le dossier de config.
    workspaces: Mutex<Option<(PathBuf, Store)>>,
}

/// Applique une modification au store des espaces, l'enregistre et le renvoie.
fn with_store(app: &AppHandle, state: &State<AppState>, f: impl FnOnce(&mut Store)) -> Result<Store, Error> {
    let mut guard = state.workspaces.lock().unwrap();
    if guard.is_none() {
        let file = app
            .path()
            .app_config_dir()
            .map_err(|e| Error::Config(e.to_string()))?
            .join("workspaces.json");
        let store = Store::load(&file);
        *guard = Some((file, store));
    }
    let (file, store) = guard.as_mut().unwrap();
    f(store);
    store.save(file)?;
    Ok(store.clone())
}

#[tauri::command]
fn workspaces(app: AppHandle, state: State<'_, AppState>) -> Result<Store, Error> {
    with_store(&app, &state, |_| {})
}

#[tauri::command]
fn workspace_add(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<Store, Error> {
    with_store(&app, &state, |s| {
        s.add_root(&path);
    })
}

#[tauri::command]
fn workspace_remove(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<Store, Error> {
    with_store(&app, &state, |s| s.remove_root(&path))
}

#[tauri::command]
fn workspace_activate(app: AppHandle, state: State<'_, AppState>, path: Option<String>) -> Result<Store, Error> {
    with_store(&app, &state, |s| s.set_active(path.as_deref()))
}

#[tauri::command]
fn workspace_forget(app: AppHandle, state: State<'_, AppState>, repo: String) -> Result<Store, Error> {
    with_store(&app, &state, |s| s.forget(&repo))
}

#[tauri::command]
async fn workspace_scan(path: String) -> Result<ScanResult, Error> {
    tauri::async_runtime::spawn_blocking(move || workspace::scan(Path::new(&path), ScanOptions::default()))
        .await
        .map_err(internal)?
}

/// Une tâche de fond qui a paniqué ou a été annulée.
fn internal(e: impl std::fmt::Display) -> Error {
    Error::Internal(e.to_string())
}

fn current(state: &State<AppState>) -> Result<Arc<Repo>, Error> {
    state.repo.lock().unwrap().clone().ok_or(Error::NoRepo)
}

#[tauri::command]
async fn open_repo(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<RepoSummary, Error> {
    let repo = tauri::async_runtime::spawn_blocking(move || Repo::open(&path))
        .await
        .map_err(internal)??;
    let repo = Arc::new(repo);
    let s = view::summary(&repo);
    let generation = {
        let mut g = state.generation.lock().unwrap();
        *g += 1;
        *g
    };
    let (workdir, git_dir) = (repo.workdir.clone(), repo.git_dir.clone());
    *state.repo.lock().unwrap() = Some(repo);
    *state.watcher.lock().unwrap() = watch(app.clone(), workdir, git_dir, generation).ok();
    let _ = with_store(&app, &state, |st| st.record_open(&s.path));
    Ok(s)
}

#[tauri::command]
async fn rows(state: State<'_, AppState>, start: u32, end: u32) -> Result<Vec<Row>, Error> {
    let repo = current(&state)?;
    Ok(view::rows(&repo, start, end))
}

#[tauri::command]
async fn edges(state: State<'_, AppState>, start: u32, end: u32) -> Result<Vec<EdgeDto>, Error> {
    let repo = current(&state)?;
    Ok(view::edges(&repo, start, end))
}

#[tauri::command]
async fn search(state: State<'_, AppState>, query: String) -> Result<SearchResult, Error> {
    Ok(current(&state)?.search.search(&query))
}

#[tauri::command]
async fn highlights(state: State<'_, AppState>, query: String, rows: Vec<u32>) -> Result<Vec<Vec<u32>>, Error> {
    Ok(current(&state)?.search.highlights(&query, &rows))
}

#[tauri::command]
async fn find_row(state: State<'_, AppState>, sha: String) -> Result<Option<u32>, Error> {
    Ok(current(&state)?.find_row(&sha))
}

#[tauri::command]
async fn commit_details(state: State<'_, AppState>, row: u32) -> Result<Details, Error> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || view::details(&repo, row))
        .await
        .map_err(internal)?
}

#[tauri::command]
async fn file_diff(state: State<'_, AppState>, row: u32, path: String, old_path: Option<String>, untracked: bool) -> Result<String, Error> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || view::file_diff(&repo, row, &path, old_path.as_deref(), untracked))
        .await
        .map_err(internal)?
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
                    let _ = app.emit("repo-error", e);
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
            workspaces,
            workspace_add,
            workspace_remove,
            workspace_activate,
            workspace_forget,
            workspace_scan,
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
        .expect("failed to start Ramure");
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
