// Accès au cœur Rust via les commandes Tauri. Hors Tauri (navigateur, tests), un backend
// de démonstration lit `public/sample.json`, produit par `npm run sample` à partir d'un vrai
// dépôt : l'interface peut ainsi être développée et testée sans l'application native.

import { invoke } from "@tauri-apps/api/core";

export type RefKind = "local" | "remote" | "tag" | "stash";
export type CommitKind = "commit" | "merge" | "stash" | "wip";

export interface RefInfo {
  name: string;
  full: string;
  kind: RefKind;
  row: number;
  head: boolean;
  synced_remote: string | null;
  merged_into_local: boolean;
  /** Branche locale extraite dans un autre worktree : chemin de ce worktree. */
  worktree: string | null;
}

export interface WorkStatus {
  staged: number;
  unstaged: number;
  untracked: number;
  conflicted: number;
  operation: string | null;
}

export interface Identity {
  name: string | null;
  email: string | null;
  origin: string | null;
  signing_key: string | null;
}

export interface RepoSummary {
  path: string;
  name: string;
  rows: number;
  max_lanes: number;
  head: { branch: string | null; row: number | null; detached: boolean };
  head_chain: number | null;
  trunk_chains: number[];
  trunk_names: string[];
  refs: RefInfo[];
  status: WorkStatus;
  identity: Identity;
  timings: Record<string, number>;
  git_version: string | null;
  /** Worktrees du dépôt, le principal en premier (un seul s'il n'y en a pas d'autre). */
  worktrees?: Worktree[];
}

export interface Worktree {
  path: string;
  name: string;
  branch: string | null;
  head: string | null;
  main: boolean;
  current: boolean;
  locked: boolean;
  prunable: boolean;
}

export interface RowRef {
  name: string;
  kind: RefKind;
  head: boolean;
  remote: string | null;
  /** Nom du worktree où cette branche est extraite, si ce n'est pas celui-ci. */
  worktree: string | null;
}

export interface Row {
  row: number;
  id: string;
  summary: string;
  author: string;
  email: string;
  time: number;
  lane: number;
  chain: number;
  color: number;
  kind: CommitKind;
  refs: RowRef[];
}

export interface Edge {
  from_row: number;
  to_row: number;
  from_lane: number;
  via_lane: number;
  to_lane: number;
  chain: number;
  color: number;
}

export interface SearchResult {
  total: number;
  rows: number[];
  elapsed_ms: number;
}

export interface FileChange {
  status: string;
  path: string;
  old_path: string | null;
  added: number | null;
  deleted: number | null;
}

export interface CommitDetails {
  type: "commit";
  id: string;
  parents: string[];
  author: string;
  author_email: string;
  author_time: number;
  committer: string;
  committer_email: string;
  committer_time: number;
  signature: string;
  message: string;
  files: FileChange[];
}

export interface WipDetails {
  type: "wip";
  files: FileChange[];
  status: WorkStatus;
}

export type Details = CommitDetails | WipDetails;

export interface RecentRepo {
  path: string;
  opened_at: number;
}

export interface WorkspaceRoot {
  path: string;
  added_at: number;
  recent: RecentRepo[];
}

export interface WorkspaceStore {
  version: number;
  active: string | null;
  roots: WorkspaceRoot[];
  recent_outside: RecentRepo[];
}

export interface TreeNode {
  name: string;
  path: string;
  kind: "dir" | "repo";
  branch: string | null;
  /** Worktree lié : nom du dépôt principal. */
  worktree_of?: string;
  children: TreeNode[];
}

export interface ScanResult {
  root: string;
  tree: TreeNode[];
  repos: number;
  truncated: boolean;
  elapsed_ms: number;
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface Api {
  initialPath(): Promise<string | null>;
  openRepo(path: string): Promise<RepoSummary>;
  rows(start: number, end: number): Promise<Row[]>;
  edges(start: number, end: number): Promise<Edge[]>;
  search(query: string): Promise<SearchResult>;
  highlights(query: string, rows: number[]): Promise<number[][]>;
  findRow(sha: string): Promise<number | null>;
  commitDetails(row: number): Promise<Details>;
  fileDiff(row: number, path: string, oldPath: string | null, untracked: boolean): Promise<string>;
  workspaces(): Promise<WorkspaceStore>;
  workspaceAdd(path: string): Promise<WorkspaceStore>;
  workspaceRemove(path: string): Promise<WorkspaceStore>;
  workspaceActivate(path: string | null): Promise<WorkspaceStore>;
  workspaceForget(repo: string): Promise<WorkspaceStore>;
  workspaceScan(path: string): Promise<ScanResult>;
  /** Modifications non commitées de chaque worktree (`null` : illisible ou disparu). */
  worktreeDirty(paths: string[]): Promise<(boolean | null)[]>;
}

const tauriApi: Api = {
  initialPath: () => invoke("initial_path"),
  openRepo: (path) => invoke("open_repo", { path }),
  rows: (start, end) => invoke("rows", { start, end }),
  edges: (start, end) => invoke("edges", { start, end }),
  search: (query) => invoke("search", { query }),
  highlights: (query, rows) => invoke("highlights", { query, rows }),
  findRow: (sha) => invoke("find_row", { sha }),
  commitDetails: (row) => invoke("commit_details", { row }),
  fileDiff: (row, path, oldPath, untracked) => invoke("file_diff", { row, path, oldPath, untracked }),
  workspaces: () => invoke("workspaces"),
  workspaceAdd: (path) => invoke("workspace_add", { path }),
  workspaceRemove: (path) => invoke("workspace_remove", { path }),
  workspaceActivate: (path) => invoke("workspace_activate", { path }),
  workspaceForget: (repo) => invoke("workspace_forget", { repo }),
  workspaceScan: (path) => invoke("workspace_scan", { path }),
  worktreeDirty: (paths) => invoke("worktree_dirty", { paths }),
};

interface Sample {
  summary: RepoSummary;
  rows: Row[];
  edges: Edge[];
  details: Record<string, Details>;
  diffs: Record<string, string>;
}

// --- Espaces de travail simulés (mode navigateur) -------------------------------------------
// Mêmes règles que le store Rust (`workspace.rs`), conservées dans le localStorage.

const WS_KEY = "ramure.demo.workspaces";
const MAX_RECENT = 20;

function readStore(): WorkspaceStore {
  try {
    const raw = localStorage.getItem(WS_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    /* stockage indisponible */
  }
  return { version: 1, active: null, roots: [], recent_outside: [] };
}
function writeStore(s: WorkspaceStore): WorkspaceStore {
  try {
    localStorage.setItem(WS_KEY, JSON.stringify(s));
  } catch {
    /* stockage indisponible : le store vit le temps de la page */
  }
  return s;
}
const inside = (repo: string, root: string) => repo === root || repo.startsWith(root.endsWith("/") ? root : root + "/");
function pushRecent(list: RecentRepo[], path: string, at: number): RecentRepo[] {
  return [{ path, opened_at: at }, ...list.filter((r) => r.path !== path)].slice(0, MAX_RECENT);
}
export const demoStore = {
  get: readStore,
  add(path: string): WorkspaceStore {
    const s = readStore();
    const p = path.replace(/\/+$/, "") || "/";
    if (!s.roots.some((r) => r.path === p)) {
      const moved = s.recent_outside.filter((r) => inside(r.path, p));
      s.recent_outside = s.recent_outside.filter((r) => !inside(r.path, p));
      s.roots.push({ path: p, added_at: Math.floor(Date.now() / 1000), recent: moved });
    }
    s.active = p;
    return writeStore(s);
  },
  remove(path: string): WorkspaceStore {
    const s = readStore();
    s.roots = s.roots.filter((r) => r.path !== path);
    if (s.active === path) s.active = s.roots[0]?.path ?? null;
    return writeStore(s);
  },
  activate(path: string | null): WorkspaceStore {
    const s = readStore();
    s.active = path && s.roots.some((r) => r.path === path) ? path : null;
    return writeStore(s);
  },
  forget(repo: string): WorkspaceStore {
    const s = readStore();
    s.roots.forEach((r) => (r.recent = r.recent.filter((x) => x.path !== repo)));
    s.recent_outside = s.recent_outside.filter((x) => x.path !== repo);
    return writeStore(s);
  },
  recordOpen(repo: string, at = Math.floor(Date.now() / 1000)): WorkspaceStore {
    const s = readStore();
    let found = false;
    for (const r of s.roots) {
      if (inside(repo, r.path)) {
        r.recent = pushRecent(r.recent, repo, at);
        found = true;
      }
    }
    if (!found) s.recent_outside = pushRecent(s.recent_outside, repo, at);
    return writeStore(s);
  },
};

/** Arbre de démonstration : le dépôt d'exemple et quelques voisins fictifs. */
function demoTree(root: string, sample: string): ScanResult {
  const repo = (name: string, path: string, branch: string | null): TreeNode => ({ name, path, kind: "repo", branch, children: [] });
  const dir = (name: string, path: string, children: TreeNode[]): TreeNode => ({ name, path, kind: "dir", branch: null, children });
  const r = root.replace(/\/+$/, "");
  return {
    root: r,
    repos: 6,
    truncated: false,
    elapsed_ms: 3.2,
    tree: [
      dir("clients / acme / apps", `${r}/clients/acme/apps`, [repo("api", `${r}/clients/acme/apps/api`, "main"), repo("site", `${r}/clients/acme/apps/site`, "develop")]),
      dir("creatiwity", `${r}/creatiwity`, [repo("incubator", `${r}/creatiwity/incubator`, "main"), repo("mesalia-home", `${r}/creatiwity/mesalia-home`, "feat/ota")]),
      repo(sample.split("/").pop() ?? "sample-repo", sample, "feature/scoring"),
      repo("dotfiles", `${r}/dotfiles`, "main"),
    ],
  };
}

/** Backend de démonstration : mêmes réponses que le cœur Rust, calculées sur un export. */
export function sampleApi(load: () => Promise<Sample>): Api {
  let data: Promise<Sample> | null = null;
  const get = () => (data ??= load());
  const words = (q: string) => q.toLowerCase().split(/\s+/).filter(Boolean);
  const match = (r: Row, q: string) => {
    const hay = `${r.summary} ${r.author} ${r.refs.map((x) => x.name).join(" ")}`.toLowerCase();
    return words(q).every((w) => (r.id.startsWith(w) && w.length >= 4) || fuzzyIncludes(hay, w));
  };
  return {
    initialPath: async () => (await get()).summary.path,
    openRepo: async (path) => {
      const d = await get();
      if (path !== d.summary.path) throw { code: "demo_only", detail: path };
      demoStore.recordOpen(path);
      return d.summary;
    },
    rows: async (s, e) => (await get()).rows.slice(s, e),
    edges: async (s, e) => (await get()).edges.filter((x) => x.from_row < e && x.to_row >= s),
    search: async (q) => {
      const t = performance.now();
      const rows = (await get()).rows.filter((r) => q.trim() !== "" && match(r, q)).map((r) => r.row);
      return { total: rows.length, rows, elapsed_ms: performance.now() - t };
    },
    highlights: async (q, rows) => {
      const all = (await get()).rows;
      return rows.map((row) => highlightIndices(all[row]?.summary ?? "", words(q)));
    },
    findRow: async (sha) => (await get()).rows.find((r) => r.id.startsWith(sha))?.row ?? null,
    commitDetails: async (row) => {
      const d = (await get()).details[String(row)];
      if (!d) throw { code: "demo_missing", detail: String(row) };
      return d;
    },
    fileDiff: async (row, path) => (await get()).diffs[`${row}:${path}`] ?? "",
    workspaces: async () => demoStore.get(),
    workspaceAdd: async (p) => demoStore.add(p),
    workspaceRemove: async (p) => demoStore.remove(p),
    workspaceActivate: async (p) => demoStore.activate(p),
    workspaceForget: async (p) => demoStore.forget(p),
    workspaceScan: async (p) => demoTree(p, (await get()).summary.path),
    worktreeDirty: async (paths) => paths.map(() => null),
  };
}

/** Sous-séquence insensible à la casse (approximation du matcher flou du cœur). */
export function fuzzyIncludes(hay: string, needle: string): boolean {
  let i = 0;
  for (const ch of hay) {
    if (ch === needle[i]) i++;
    if (i === needle.length) return true;
  }
  return needle.length === 0;
}

export function highlightIndices(text: string, ws: string[]): number[] {
  const chars = [...text.toLowerCase()];
  const out = new Set<number>();
  for (const w of ws) {
    let i = 0;
    const found: number[] = [];
    for (let k = 0; k < chars.length && i < w.length; k++) {
      if (chars[k] === w[i]) {
        found.push(k);
        i++;
      }
    }
    if (i === w.length) found.forEach((k) => out.add(k));
  }
  return [...out].sort((a, b) => a - b);
}

export const api: Api = inTauri
  ? tauriApi
  : sampleApi(async () => {
      const res = await fetch("/sample.json");
      if (!res.ok) throw { code: "demo_sample", detail: "public/sample.json" };
      return res.json();
    });
