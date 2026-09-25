<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import { api, inTauri, type FileChange, type RepoSummary, type ScanResult, type TreeNode, type WorkStatus, type WorkspaceStore } from "./api";
import { ago, basename, relativeTo, usesConventionalCommits } from "./lib/format";
import GraphView from "./components/GraphView.vue";
import Sidebar from "./components/Sidebar.vue";
import DetailsPanel from "./components/DetailsPanel.vue";
import DiffView from "./components/DiffView.vue";
import Icon from "./components/Icon.vue";
import WorkspacePanel from "./components/WorkspacePanel.vue";
import CommandPalette, { type PaletteItem } from "./components/CommandPalette.vue";

const summary = shallowRef<RepoSummary | null>(null);
const revision = ref(0);
const selected = ref<number | null>(null);
const opening = ref(false);
const error = ref<string | null>(null);
const conventional = ref(false);
const rainbow = ref(load("ramure.rainbow") === "1");
const theme = ref<string>(load("ramure.theme") ?? "");

// Espaces de travail : dossiers racines, dépôts récents par contexte, arbre du contexte actif.
const ws = shallowRef<WorkspaceStore>({ version: 1, active: null, roots: [], recent_outside: [] });
const wsScan = shallowRef<ScanResult | null>(null);
const scanning = ref(false);
const panelOpen = ref(load("ramure.panel") !== "0");
const paletteOpen = ref(false);
const canPick = inTauri;

const query = ref("");
const hits = shallowRef<Set<number> | null>(null);
const hitList = shallowRef<number[]>([]);
const hitIndex = ref(-1);
const searchInfo = ref<{ total: number; ms: number } | null>(null);
const searchInput = ref<HTMLInputElement>();
const graph = ref<InstanceType<typeof GraphView>>();
const diffFile = ref<FileChange | null>(null);

function load(k: string): string | null {
  try {
    return localStorage.getItem(k);
  } catch {
    return null;
  }
}
function save(k: string, v: string) {
  try {
    localStorage.setItem(k, v);
  } catch {
    /* stockage indisponible : préférence non mémorisée */
  }
}
function applyTheme() {
  if (theme.value) document.documentElement.setAttribute("data-theme", theme.value);
  else document.documentElement.removeAttribute("data-theme");
}
function cycleTheme() {
  theme.value = theme.value === "" ? "light" : theme.value === "light" ? "dark" : "";
  save("ramure.theme", theme.value);
  applyTheme();
}
const themeLabel = computed(() => (theme.value === "light" ? "Clair" : theme.value === "dark" ? "Sombre" : "Système"));
function toggleRainbow() {
  rainbow.value = !rainbow.value;
  save("ramure.rainbow", rainbow.value ? "1" : "0");
}

async function open(path: string) {
  opening.value = true;
  error.value = null;
  try {
    const s = await api.openRepo(path);
    summary.value = s;
    revision.value++;
    selected.value = s.head.row ?? (s.rows ? 0 : null);
    resetSearch();
    ws.value = await api.workspaces();
    document.title = `${s.name} — Ramure`;
    if (inTauri) import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().setTitle(document.title)).catch(() => {});
    const sample = await api.rows(0, Math.min(s.rows, 500));
    conventional.value = usesConventionalCommits(sample.filter((r) => r.kind === "commit").map((r) => r.summary));
    requestAnimationFrame(() => {
      if (s.head.row != null) graph.value?.reveal(s.head.row, true);
      graph.value?.focus();
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    opening.value = false;
  }
}

// --- Espaces de travail -------------------------------------------------------
let scanToken = 0;
async function rescan() {
  const root = ws.value.active;
  const t = ++scanToken;
  if (!root) {
    wsScan.value = null;
    return;
  }
  scanning.value = true;
  try {
    const res = await api.workspaceScan(root);
    if (t === scanToken) wsScan.value = res;
  } catch (e) {
    if (t === scanToken) wsScan.value = { root, tree: [], repos: 0, truncated: false, elapsed_ms: 0 };
    error.value = String(e);
  } finally {
    if (t === scanToken) scanning.value = false;
  }
}
async function addRoot() {
  if (!inTauri) return;
  const { open: dialog } = await import("@tauri-apps/plugin-dialog");
  const dir = await dialog({ directory: true, title: "Choisir un dossier racine (ses dépôts seront listés)" });
  if (typeof dir !== "string") return;
  ws.value = await api.workspaceAdd(dir);
  panelOpen.value = true;
  rescan();
}
async function removeRoot(root: string) {
  const before = ws.value.active;
  ws.value = await api.workspaceRemove(root);
  if (ws.value.active !== before) rescan();
}
async function activateRoot(root: string) {
  ws.value = await api.workspaceActivate(root);
  rescan();
}
async function forgetRepo(repo: string) {
  ws.value = await api.workspaceForget(repo);
}
function togglePanel() {
  panelOpen.value = !panelOpen.value;
  save("ramure.panel", panelOpen.value ? "1" : "0");
}
/** Récents du contexte actif (ou hors racine s'il n'y en a pas) : cibles de ⌘1…⌘9. */
const activeRecents = computed(() => {
  const root = ws.value.roots.find((r) => r.path === ws.value.active);
  return (root ? root.recent : ws.value.recent_outside).slice(0, 9);
});
const modKey = navigator.platform.includes("Mac") ? "⌘" : "Ctrl+";

// --- Palette de commandes ------------------------------------------------------
const paletteItems = computed<PaletteItem[]>(() => {
  const items: PaletteItem[] = [];
  const seen = new Set<string>();
  const activeRoot = ws.value.active;
  // 1. Dépôts récents : contexte actif d'abord (avec ⌘1…⌘9), puis les autres contextes.
  const contexts = [...ws.value.roots].sort((a, b) => (a.path === activeRoot ? -1 : b.path === activeRoot ? 1 : 0));
  contexts.forEach((root) =>
    root.recent.forEach((r, i) => {
      if (seen.has(r.path)) return;
      seen.add(r.path);
      items.push({
        id: `recent:${r.path}`,
        group: "Dépôts récents",
        label: basename(r.path),
        detail: `${basename(root.path)} · ${relativeTo(r.path, root.path)} · ${ago(r.opened_at)}`,
        search: `${basename(r.path)} ${relativeTo(r.path, root.path)}`,
        icon: "repo",
        hint: root.path === activeRoot && i < 9 ? `${modKey}${i + 1}` : undefined,
        run: () => openRepo(r.path),
      });
    }),
  );
  ws.value.recent_outside.forEach((r) => {
    if (seen.has(r.path)) return;
    seen.add(r.path);
    items.push({ id: `recent:${r.path}`, group: "Dépôts récents", label: basename(r.path), detail: `${r.path} · ${ago(r.opened_at)}`, icon: "repo", run: () => openRepo(r.path) });
  });
  // 2. Dépôts trouvés dans le contexte actif.
  const walk = (nodes: TreeNode[]) =>
    nodes.forEach((n) => {
      if (n.kind === "dir") return walk(n.children);
      if (seen.has(n.path)) return;
      seen.add(n.path);
      const rel = relativeTo(n.path, wsScan.value?.root ?? null);
      items.push({ id: `repo:${n.path}`, group: `Dépôts de ${basename(activeRoot ?? "")}`, label: basename(n.path), detail: n.branch ? `${rel} · ${n.branch}` : rel, search: rel, icon: "repo", run: () => openRepo(n.path) });
    });
  walk(wsScan.value?.tree ?? []);
  // 3. Branches et tags du dépôt ouvert.
  for (const r of summary.value?.refs ?? []) {
    if (r.merged_into_local) continue;
    const kind = r.kind === "local" ? "branche locale" : r.kind === "remote" ? "branche distante" : r.kind === "tag" ? "tag" : "stash";
    items.push({ id: `ref:${r.full}`, group: "Branches et tags", label: r.name, detail: r.head ? `${kind} · HEAD` : kind, icon: r.kind === "tag" ? "tag" : r.kind === "remote" ? "cloud" : r.kind === "stash" ? "box" : "branch", run: () => gotoRow(r.row) });
  }
  // 4. Autres contextes.
  for (const root of ws.value.roots) {
    if (root.path === activeRoot) continue;
    items.push({ id: `ctx:${root.path}`, group: "Espaces", label: `Passer à ${basename(root.path)}`, detail: root.path, search: `espace ${basename(root.path)}`, icon: "folder", run: () => activateRoot(root.path) });
  }
  // 5. Actions.
  const actions: [string, string, string | undefined, () => void][] = [
    ["Ouvrir un dépôt…", "folder", `${modKey}O`, pickFolder],
    ["Ajouter un dossier racine…", "plus", undefined, addRoot],
    [panelOpen.value ? "Masquer le panneau des espaces" : "Afficher le panneau des espaces", "panel", `${modKey}⇧E`, togglePanel],
    ["Rechercher dans l'historique", "search", `${modKey}F`, () => searchInput.value?.focus()],
    ["Rechercher à nouveau les dépôts", "refresh", undefined, rescan],
    [rainbow.value ? "Couleurs : focus" : "Couleurs : arc-en-ciel", "palette", undefined, toggleRainbow],
    [`Thème : ${theme.value === "" ? "clair" : theme.value === "light" ? "sombre" : "système"}`, "sun", undefined, cycleTheme],
  ];
  if (summary.value?.head.row != null) {
    const headRow = summary.value.head.row;
    actions.splice(3, 0, ["Aller à HEAD", "check", "H", () => gotoRow(headRow)]);
  }
  actions.forEach(([label, icon, hint, run]) => items.push({ id: `act:${label}`, group: "Actions", label, icon, hint, run }));
  return items;
});

/** Ouvre un dépôt ; s'il n'existe plus sur le disque, il est retiré des récents. */
async function openRepo(path: string) {
  await open(path);
  if (error.value?.includes("Dossier introuvable")) {
    await forgetRepo(path);
    error.value += " Il a été retiré des récents.";
  }
}

async function pickFolder() {
  if (!inTauri) return;
  const { open: dialog } = await import("@tauri-apps/plugin-dialog");
  const dir = await dialog({ directory: true, title: "Ouvrir un dépôt git" });
  if (typeof dir === "string") open(dir);
}

// --- Recherche --------------------------------------------------------------
let searchToken = 0;
async function runSearch() {
  const q = query.value;
  const t = ++searchToken;
  if (!q.trim() || !summary.value) {
    resetSearch(false);
    return;
  }
  const res = await api.search(q);
  if (t !== searchToken) return;
  hitList.value = res.rows;
  hits.value = new Set(res.rows);
  searchInfo.value = { total: res.total, ms: res.elapsed_ms };
  const from = selected.value ?? 0;
  hitIndex.value = res.rows.findIndex((r) => r >= from);
  if (hitIndex.value < 0 && res.rows.length) hitIndex.value = 0;
  if (hitIndex.value >= 0) goHit(hitIndex.value);
}
function resetSearch(clearQuery = true) {
  if (clearQuery) query.value = "";
  hits.value = null;
  hitList.value = [];
  hitIndex.value = -1;
  searchInfo.value = null;
}
function goHit(i: number) {
  if (!hitList.value.length) return;
  hitIndex.value = (i + hitList.value.length) % hitList.value.length;
  const row = hitList.value[hitIndex.value];
  selected.value = row;
  graph.value?.reveal(row, true);
}
function onSearchKey(e: KeyboardEvent) {
  if (e.key === "Enter") {
    e.preventDefault();
    goHit(hitIndex.value + (e.shiftKey ? -1 : 1));
  } else if (e.key === "Escape") {
    resetSearch();
    graph.value?.focus();
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    graph.value?.focus();
  }
}

async function gotoSha(sha: string) {
  const row = await api.findRow(sha);
  if (row != null) {
    selected.value = row;
    graph.value?.reveal(row, true);
  }
}
function gotoRow(row: number) {
  selected.value = row;
  graph.value?.reveal(row, true);
}

const selectedIsWip = computed(() => summary.value != null && selected.value === 0 && summary.value.status && isDirty(summary.value.status));
function isDirty(s: WorkStatus) {
  return s.staged + s.unstaged + s.untracked + s.conflicted > 0;
}

function onGlobalKey(e: KeyboardEvent) {
  const mod = e.metaKey || e.ctrlKey;
  if (mod && e.key.toLowerCase() === "f") {
    e.preventDefault();
    searchInput.value?.focus();
    searchInput.value?.select();
  } else if (mod && e.key.toLowerCase() === "o") {
    e.preventDefault();
    pickFolder();
  } else if (mod && e.key.toLowerCase() === "k") {
    e.preventDefault();
    paletteOpen.value = !paletteOpen.value;
  } else if (mod && e.shiftKey && e.key.toLowerCase() === "e") {
    e.preventDefault();
    togglePanel();
  } else if (mod && !e.shiftKey && /^[1-9]$/.test(e.key)) {
    const r = activeRecents.value[Number(e.key) - 1];
    if (r) {
      e.preventDefault();
      openRepo(r.path);
    }
  }
}

const unlisten: (() => void)[] = [];
onMounted(async () => {
  applyTheme();
  window.addEventListener("keydown", onGlobalKey);
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    unlisten.push(
      await listen<RepoSummary>("repo-changed", async (ev) => {
        // Conserver la sélection sur le même commit après un rechargement.
        const keepId = selected.value != null ? (await api.rows(selected.value, selected.value + 1).catch(() => []))[0]?.id : undefined;
        summary.value = ev.payload;
        revision.value++;
        if (query.value) runSearch();
        if (keepId) {
          const row = await api.findRow(keepId);
          selected.value = row ?? ev.payload.head.row;
        }
      }),
      await listen<WorkStatus>("status-changed", (ev) => {
        if (summary.value) summary.value = { ...summary.value, status: ev.payload };
      }),
      await getCurrentWebview().onDragDropEvent((ev) => {
        if (ev.payload.type === "drop" && ev.payload.paths.length) open(ev.payload.paths[0]);
      }),
    );
  }
  ws.value = await api.workspaces().catch(() => ws.value);
  rescan();
  const initial = await api.initialPath().catch(() => null);
  if (initial) open(initial);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKey);
  unlisten.forEach((u) => u());
});

const statusText = computed(() => {
  const s = summary.value;
  if (!s) return "";
  const t = s.timings.total_ms;
  return `${s.rows.toLocaleString("fr-FR")} commits · chargé en ${t < 1000 ? Math.round(t) + " ms" : (t / 1000).toFixed(1) + " s"}`;
});
</script>

<template>
  <div class="app">
    <header class="tb">
      <button class="tool icon" :class="{ on: panelOpen }" :title="`${panelOpen ? 'Masquer' : 'Afficher'} le panneau des espaces (${modKey}⇧E)`" @click="togglePanel">
        <Icon name="panel" />
      </button>
      <template v-if="summary">
        <button class="crumb" :title="`Ouvrir un autre dépôt (${modKey}O)`" @click="pickFolder">
          <Icon name="folder" /><b>{{ summary.name }}</b>
          <span class="br"><Icon name="branch" />{{ summary.head.branch ?? (summary.head.detached ? "HEAD détaché" : "—") }}</span>
        </button>
        <span class="sep"></span>
        <span v-if="summary.identity.email" class="idchip" :title="`Identité git effective\n${summary.identity.name ?? ''} <${summary.identity.email}>\nDéfinie dans : ${summary.identity.origin ?? '?'}`">
          <Icon name="user" />{{ summary.identity.email }}<span class="origin">{{ summary.identity.origin?.replace(/^.*\//, "") }}</span>
        </span>
        <span v-else class="idchip warn" title="Aucune identité git configurée pour ce dépôt"><Icon name="warn" />Pas d'identité git</span>
        <span v-if="summary.status.operation" class="opchip"><Icon name="warn" />{{ summary.status.operation }} en cours</span>
      </template>
      <label class="search" :class="{ on: query }">
        <Icon name="search" />
        <input
          id="search"
          ref="searchInput"
          v-model="query"
          :disabled="!summary"
          placeholder="Rechercher : message, sha, auteur, ref…"
          spellcheck="false"
          autocomplete="off"
          @input="runSearch"
          @keydown="onSearchKey"
        />
        <span v-if="searchInfo" class="count">
          {{ searchInfo.total ? `${hitIndex + 1} / ${searchInfo.total.toLocaleString("fr-FR")}` : "aucun résultat" }} · {{ searchInfo.ms.toFixed(1) }} ms
        </span>
        <kbd v-else>{{ modKey }}F</kbd>
      </label>
      <button class="tool" :title="`Palette de commandes (${modKey}K)`" @click="paletteOpen = true"><Icon name="cmd" />{{ modKey }}K</button>
      <button class="tool" :title="rainbow ? 'Couleurs : arc-en-ciel' : 'Couleurs : focus (branche courante et troncs)'" @click="toggleRainbow">
        <Icon name="palette" />{{ rainbow ? "Arc-en-ciel" : "Focus" }}
      </button>
      <button class="tool" title="Thème" @click="cycleTheme"><Icon name="sun" />{{ themeLabel }}</button>
    </header>

    <div class="shell" :class="{ withPanel: panelOpen }">
    <WorkspacePanel
      v-if="panelOpen"
      :store="ws"
      :scan="wsScan"
      :scanning="scanning"
      :current="summary?.path ?? null"
      :can-pick="canPick"
      @open="openRepo"
      @add="addRoot"
      @remove="removeRoot"
      @activate="activateRoot"
      @rescan="rescan"
      @forget="forgetRepo"
      @collapse="togglePanel"
    />
    <main v-if="summary" class="body">
      <Sidebar :summary="summary" @goto="gotoRow" />
      <div class="center">
        <GraphView
          ref="graph"
          :summary="summary"
          :selected="selected"
          :hits="hits"
          :query="query"
          :conventional="conventional"
          :rainbow="rainbow"
          :revision="revision"
          @select="(r) => ((selected = r), (diffFile = null))"
          @open="() => {}"
        />
        <DiffView v-if="diffFile && selected != null" :row="selected" :file="diffFile" :wip="!!selectedIsWip" @close="diffFile = null" />
      </div>
      <DetailsPanel :row="selected" :revision="revision" @diff="(f) => (diffFile = f)" @goto="gotoSha" />
    </main>

    <main v-else class="welcome">
      <div class="card">
        <h1>Ramure</h1>
        <p>Viewer git en lecture seule. Ouvrez un dépôt, ou glissez son dossier sur cette fenêtre.</p>
        <button class="btn primary" :disabled="opening || !inTauri" @click="pickFolder"><Icon name="folder" />Ouvrir un dépôt… <kbd>{{ modKey }}O</kbd></button>
        <p v-if="!inTauri" class="muted">Mode navigateur : données d'exemple (<span class="mono">public/sample.json</span>).</p>
        <p v-if="error" class="err">{{ error }}</p>
        <div v-if="activeRecents.length" class="recents">
          <span class="eyebrow">Récents</span>
          <button v-for="(r, i) in activeRecents" :key="r.path" class="recent" @click="openRepo(r.path)">
            <Icon name="repo" /><b>{{ basename(r.path) }}</b><span class="rp">{{ relativeTo(r.path, ws.active) }}</span><span class="when">{{ ago(r.opened_at) }}</span
            ><kbd>{{ modKey }}{{ i + 1 }}</kbd>
          </button>
        </div>
        <p v-else-if="!ws.roots.length" class="muted">Astuce : ajoutez un dossier racine (par exemple ~/code) dans le panneau de gauche pour retrouver vos dépôts en un clic.</p>
        <p class="muted"><kbd>{{ modKey }}K</kbd> ouvre la palette : dépôts récents, dépôts du dossier racine, actions.</p>
        <p v-if="opening" class="muted">Ouverture…</p>
      </div>
    </main>
    </div>

    <div v-if="error && summary" class="toast" role="alert">
      <Icon name="warn" /><span>{{ error }}</span><button class="tool" @click="error = null"><Icon name="x" /></button>
    </div>

    <CommandPalette v-if="paletteOpen" :items="paletteItems" @close="paletteOpen = false" />

    <footer v-if="summary" class="status">
      <span>{{ statusText }}</span>
      <span v-if="summary.trunk_names.length">Troncs : {{ summary.trunk_names.join(", ") }}</span>
      <span>Couleurs : {{ rainbow ? "arc-en-ciel" : "focus" }}</span>
      <span class="r">Lecture seule · git {{ summary.git_version ?? "?" }} · gix</span>
    </footer>
  </div>
</template>

<style scoped>
.app {
  height: 100%;
  display: grid;
  grid-template-rows: 44px minmax(0, 1fr) auto;
}
.tb {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
  min-width: 0;
}
.crumb {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 8px;
  border-radius: 6px;
  border: 0;
  background: none;
  cursor: pointer;
  white-space: nowrap;
}
.crumb:hover {
  background: var(--hover);
}
.crumb .br {
  display: flex;
  align-items: center;
  gap: 5px;
  color: var(--ink-2);
}
.sep {
  width: 1px;
  height: 20px;
  background: var(--line);
}
.idchip,
.opchip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px 3px 8px;
  border-radius: 14px;
  border: 1px solid var(--line);
  background: var(--bg);
  font-size: 11.5px;
  white-space: nowrap;
  overflow: hidden;
  min-width: 0;
}
.idchip .origin {
  color: var(--ink-3);
  font: 10.5px var(--f-mono);
}
.idchip.warn,
.opchip {
  color: var(--warn);
  background: var(--warn-bg);
  border-color: color-mix(in srgb, var(--warn) 40%, transparent);
}
.search {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 28px;
  padding: 0 8px;
  width: min(460px, 40vw);
  border-radius: 7px;
  border: 1px solid var(--line);
  background: var(--bg);
  color: var(--ink-3);
}
.search:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 22%, transparent);
}
.search input {
  flex: 1;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--ink);
  font: inherit;
  min-width: 0;
}
.count {
  font: 11px var(--f-mono);
  white-space: nowrap;
}
.tool {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 8px;
  border-radius: 6px;
  border: 0;
  background: none;
  color: var(--ink-2);
  cursor: pointer;
  font-size: 11.5px;
  white-space: nowrap;
}
.tool:hover {
  background: var(--hover);
  color: var(--ink);
}
.toast {
  position: fixed;
  left: 50%;
  bottom: 40px;
  transform: translateX(-50%);
  z-index: 40;
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: min(640px, 90vw);
  padding: 8px 8px 8px 12px;
  border-radius: 9px;
  background: var(--warn-bg);
  color: var(--ink);
  border: 1px solid color-mix(in srgb, var(--warn) 45%, transparent);
  box-shadow: var(--shadow);
  user-select: text;
}
.toast > .i {
  color: var(--warn);
}
.shell {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  min-height: 0;
}
.shell.withPanel {
  grid-template-columns: 260px minmax(0, 1fr);
}
.body {
  display: grid;
  grid-template-columns: 210px minmax(0, 1fr) 320px;
  min-height: 0;
}
.tool.icon {
  padding: 5px;
}
.tool.on {
  color: var(--accent);
}
.center {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}
.center > :first-child {
  flex: 1;
}
.status {
  height: 24px;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 12px;
  border-top: 1px solid var(--line);
  background: var(--panel);
  color: var(--ink-3);
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
}
.status .r {
  margin-left: auto;
}
.welcome {
  display: grid;
  place-items: center;
}
.card {
  width: min(520px, 90vw);
  display: flex;
  flex-direction: column;
  gap: 14px;
  align-items: flex-start;
}
.card h1 {
  margin: 0;
  font-size: 34px;
  letter-spacing: -0.01em;
}
.card p {
  margin: 0;
  color: var(--ink-2);
}
.muted {
  color: var(--ink-3) !important;
}
.err {
  color: var(--del) !important;
  user-select: text;
}
.recents {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  margin-top: 8px;
}
.recent {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 6px 8px;
  border: 0;
  background: none;
  border-radius: 6px;
  cursor: pointer;
  text-align: left;
  color: var(--ink-2);
  font-family: var(--f-mono);
  font-size: 11.5px;
}
.recent b {
  font-family: var(--f-ui);
  color: var(--ink);
  font-weight: 600;
}
.recent .rp {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.recent .when {
  font-family: var(--f-ui);
  color: var(--ink-3);
  white-space: nowrap;
}
.recent:hover {
  background: var(--hover);
  color: var(--ink);
}
</style>
