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
}

export interface RowRef {
  name: string;
  kind: RefKind;
  head: boolean;
  remote: string | null;
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
};

interface Sample {
  summary: RepoSummary;
  rows: Row[];
  edges: Edge[];
  details: Record<string, Details>;
  diffs: Record<string, string>;
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
    openRepo: async () => (await get()).summary,
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
      if (!d) throw new Error("détails non inclus dans l'échantillon");
      return d;
    },
    fileDiff: async (row, path) => (await get()).diffs[`${row}:${path}`] ?? "",
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
      if (!res.ok) throw new Error("public/sample.json absent : lancez `npm run sample`");
      return res.json();
    });
