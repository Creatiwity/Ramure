<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { api, inTauri, type FileChange, type RepoSummary, type ScanResult, type TreeNode, type WorkStatus, type WorkspaceStore } from "./api";
import { useI18n } from "vue-i18n";
import { ago, basename, formatNumber, relativeTo, usesConventionalCommits, type Locale } from "./lib/format";
import { errorText, loadLangPref, setLangPref, systemLang, type LangPref } from "./i18n";
import GraphView from "./components/GraphView.vue";
import Sidebar from "./components/Sidebar.vue";
import DetailsPanel from "./components/DetailsPanel.vue";
import DiffView from "./components/DiffView.vue";
import Icon from "./components/Icon.vue";
import WorkspacePanel from "./components/WorkspacePanel.vue";
import CommandPalette, { type PaletteItem } from "./components/CommandPalette.vue";
import UpdateBanner from "./components/UpdateBanner.vue";
import { createUpdater, noBackend, tauriBackend } from "./lib/updates";

const { t, locale } = useI18n();
const loc = computed(() => locale.value as Locale);
const langPref = ref<LangPref>(loadLangPref());
const langMenu = ref(false);
function chooseLang(p: LangPref) {
  langPref.value = p;
  langMenu.value = false;
  setLangPref(p);
}
const langName = (l: Locale) => t(`lang.${l}`);

const summary = shallowRef<RepoSummary | null>(null);
const revision = ref(0);
const selected = ref<number | null>(null);
/** Sha du commit sélectionné, lu au moment de la sélection : après un rechargement, les numéros
 *  de ligne ont bougé, il faut retrouver le commit par son sha. */
let selectedId: string | undefined;
watch(selected, async (row) => {
  selectedId = undefined;
  if (row == null) return;
  const id = (await api.rows(row, row + 1).catch(() => []))[0]?.id;
  if (selected.value === row) selectedId = id || undefined;
});
const opening = ref(false);
const error = ref<string | null>(null);
/** Code de la dernière erreur du cœur (`not_found`, `open`…), pour réagir sans lire le texte. */
const errorCode = ref<string | null>(null);
function fail(e: unknown) {
  error.value = errorText(e);
  errorCode.value = e && typeof e === "object" && "code" in e ? String((e as { code: string }).code) : null;
}
const langBox = ref<HTMLDivElement>();
function onDocDown(e: MouseEvent) {
  if (langMenu.value && langBox.value && !langBox.value.contains(e.target as Node)) langMenu.value = false;
}
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
/** Worktrees modifiés (hors celui ouvert), lus après le chargement pour ne pas le ralentir. */
const wtDirty = shallowRef<Record<string, boolean | null>>({});
async function refreshWorktrees(s: RepoSummary) {
  const others = (s.worktrees ?? []).filter((w) => !w.current && !w.prunable);
  if (!others.length) return void (wtDirty.value = {});
  const res = await api.worktreeDirty(others.map((w) => w.path)).catch(() => others.map(() => null));
  if (summary.value?.path !== s.path) return;
  wtDirty.value = Object.fromEntries(others.map((w, i) => [w.path, res[i] ?? null]));
}

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
function remove(k: string) {
  try {
    localStorage.removeItem(k);
  } catch {
    /* stockage indisponible */
  }
}
// Mises à jour de l'application (spec §4.3, planche M).
const updater = createUpdater(inTauri ? tauriBackend : noBackend, { get: load, set: (k, v) => (v == null ? remove(k) : save(k, v)) });
/** Dépôt à rouvrir après la relance qui suit une mise à jour. */
const K_REOPEN = "ramure.reopen";
function installUpdate() {
  updater.install(() => {
    if (summary.value) save(K_REOPEN, summary.value.path);
  });
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
const themeLabel = computed(() => t(theme.value === "light" ? "toolbar.themeLight" : theme.value === "dark" ? "toolbar.themeDark" : "toolbar.themeSystem"));
function toggleRainbow() {
  rainbow.value = !rainbow.value;
  save("ramure.rainbow", rainbow.value ? "1" : "0");
}

async function open(path: string) {
  opening.value = true;
  error.value = null;
  errorCode.value = null;
  try {
    const s = await api.openRepo(path);
    summary.value = s;
    refreshWorktrees(s);
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
    fail(e);
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
    fail(e);
  } finally {
    if (t === scanToken) scanning.value = false;
  }
}
async function addRoot() {
  if (!inTauri) return;
  const { open: dialog } = await import("@tauri-apps/plugin-dialog");
  const dir = await dialog({ directory: true, title: t("app.dialogRoot") });
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
        group: t("palette.groupRecent"),
        label: basename(r.path),
        detail: `${basename(root.path)} · ${relativeTo(r.path, root.path)} · ${ago(r.opened_at, loc.value)}`,
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
    items.push({ id: `recent:${r.path}`, group: t("palette.groupRecent"), label: basename(r.path), detail: `${r.path} · ${ago(r.opened_at, loc.value)}`, icon: "repo", run: () => openRepo(r.path) });
  });
  // 2. Dépôts trouvés dans le contexte actif.
  const walk = (nodes: TreeNode[]) =>
    nodes.forEach((n) => {
      if (n.kind === "dir") return walk(n.children);
      if (seen.has(n.path)) return;
      seen.add(n.path);
      const rel = relativeTo(n.path, wsScan.value?.root ?? null);
      items.push({ id: `repo:${n.path}`, group: t("palette.groupRepos", { root: basename(activeRoot ?? "") }), label: basename(n.path), detail: n.branch ? `${rel} · ${n.branch}` : rel, search: rel, icon: "repo", run: () => openRepo(n.path) });
    });
  walk(wsScan.value?.tree ?? []);
  // 3. Branches et tags du dépôt ouvert.
  for (const r of summary.value?.refs ?? []) {
    if (r.merged_into_local) continue;
    const kind = t(r.kind === "local" ? "palette.refLocal" : r.kind === "remote" ? "palette.refRemote" : r.kind === "tag" ? "palette.refTag" : "palette.refStash");
    items.push({ id: `ref:${r.full}`, group: t("palette.groupRefs"), label: r.name, detail: r.head ? `${kind} · HEAD` : kind, icon: r.kind === "tag" ? "tag" : r.kind === "remote" ? "cloud" : r.kind === "stash" ? "box" : "branch", run: () => gotoRow(r.row) });
  }
  // 4. Autres worktrees du dépôt ouvert.
  for (const w of summary.value?.worktrees ?? []) {
    if (w.current || w.prunable || (summary.value?.worktrees?.length ?? 0) < 2) continue;
    items.push({ id: `wt:${w.path}`, group: t("palette.groupWorktrees"), label: `${w.name} · ${w.branch ?? t("sidebar.wtDetached")}`, detail: w.path, search: `${w.name} ${w.branch ?? ""}`, icon: "wt", run: () => openRepo(w.path) });
  }
  // 5. Autres contextes.
  for (const root of ws.value.roots) {
    if (root.path === activeRoot) continue;
    items.push({ id: `ctx:${root.path}`, group: t("palette.groupContexts"), label: t("palette.switchTo", { name: basename(root.path) }), detail: root.path, search: t("palette.switchSearch", { name: basename(root.path) }), icon: "folder", run: () => activateRoot(root.path) });
  }
  // 6. Actions.
  const actions: [string, string, string | undefined, () => void][] = [
    [t("palette.actOpen"), "folder", `${modKey}O`, pickFolder],
    [t("palette.actAddRoot"), "plus", undefined, addRoot],
    [t(panelOpen.value ? "palette.actHidePanel" : "palette.actShowPanel"), "panel", `${modKey}⇧E`, togglePanel],
    [t("palette.actSearch"), "search", `${modKey}F`, () => searchInput.value?.focus()],
    [t("palette.actRescan"), "refresh", undefined, rescan],
    [t(rainbow.value ? "palette.actFocus" : "palette.actRainbow"), "palette", undefined, toggleRainbow],
    [t("palette.actTheme", { name: t(theme.value === "" ? "toolbar.themeLight" : theme.value === "light" ? "toolbar.themeDark" : "toolbar.themeSystem") }), "sun", undefined, cycleTheme],
  ];
  // Changement de langue : les deux autres choix.
  for (const p of ["", "fr", "en"] as LangPref[]) {
    if (p === langPref.value) continue;
    const name = p ? langName(p) : t("lang.systemDetected", { lang: langName(systemLang()) });
    actions.push([t("palette.actLanguage", { name }), "globe", undefined, () => chooseLang(p)]);
  }
  if (summary.value?.head.row != null) {
    const headRow = summary.value.head.row;
    actions.splice(3, 0, [t("palette.actHead"), "check", "H", () => gotoRow(headRow)]);
  }
  actions.forEach(([label, icon, hint, run]) => items.push({ id: `act:${label}`, group: t("palette.groupActions"), label, icon, hint, run }));
  if (updater.supported.value) {
    const last = updater.lastCheck.value;
    items.push({
      id: "act:updates-check",
      group: t("palette.groupActions"),
      label: t("updates.actCheck"),
      detail: last ? t("updates.lastCheck", { when: ago(last, loc.value) }) : t("updates.neverChecked"),
      icon: "refresh",
      run: () => updater.check(true),
    });
    const on = updater.auto.value;
    items.push({
      id: "act:updates-auto",
      group: t("palette.groupActions"),
      label: t(on ? "updates.actAutoOff" : "updates.actAutoOn"),
      detail: t(on ? "updates.actAutoOffDetail" : "updates.actAutoOnDetail"),
      icon: on ? "x" : "check",
      run: () => updater.setAuto(!on),
    });
  }
  return items;
});

/** Ouvre un dépôt ; s'il n'existe plus sur le disque, il est retiré des récents. */
async function openRepo(path: string) {
  await open(path);
  if (errorCode.value === "not_found") {
    await forgetRepo(path);
    error.value += " " + t("app.removedFromRecents");
  }
}

async function pickFolder() {
  if (!inTauri) return;
  const { open: dialog } = await import("@tauri-apps/plugin-dialog");
  const dir = await dialog({ directory: true, title: t("app.dialogOpen") });
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
const updateTimers: ReturnType<typeof setTimeout>[] = [];
onMounted(async () => {
  applyTheme();
  window.addEventListener("keydown", onGlobalKey);
  document.addEventListener("mousedown", onDocDown);
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    unlisten.push(
      await listen<RepoSummary>("repo-changed", async (ev) => {
        // Conserver la sélection sur le même commit après un rechargement.
        const keepId = selectedId;
        summary.value = ev.payload;
        refreshWorktrees(ev.payload);
        revision.value++;
        if (query.value) runSearch();
        if (keepId) {
          const row = await api.findRow(keepId);
          selected.value = row ?? ev.payload.head.row;
        }
      }),
      await listen<unknown>("repo-error", (ev) => fail(ev.payload)),
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
  const reopen = load(K_REOPEN);
  remove(K_REOPEN);
  if (initial ?? reopen) open((initial ?? reopen)!);
  // Première vérification quelques secondes après le démarrage, puis toutes les heures on
  // regarde si la dernière date de plus de 24 h.
  await updater.init();
  updateTimers.push(setTimeout(() => updater.tick(), 5000), setInterval(() => updater.tick(), 3600_000));
});
onBeforeUnmount(() => {
  updateTimers.forEach((id) => clearTimeout(id));
  window.removeEventListener("keydown", onGlobalKey);
  document.removeEventListener("mousedown", onDocDown);
  unlisten.forEach((u) => u());
});

const statusText = computed(() => {
  const s = summary.value;
  if (!s) return "";
  const ms = s.timings.total_ms;
  const time = ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toLocaleString(loc.value, { maximumFractionDigits: 1 })} s`;
  return t("status.loaded", { n: formatNumber(s.rows, loc.value), time }, s.rows);
});
</script>

<template>
  <div class="app">
    <header class="tb">
      <button class="tool icon" :class="{ on: panelOpen }" :title="t(panelOpen ? 'toolbar.hidePanel' : 'toolbar.showPanel', { key: `${modKey}⇧E` })" @click="togglePanel">
        <Icon name="panel" />
      </button>
      <template v-if="summary">
        <button class="crumb" :title="t('app.openAnother', { key: `${modKey}O` })" @click="pickFolder">
          <Icon name="folder" /><b>{{ summary.name }}</b>
          <span class="br"><Icon name="branch" />{{ summary.head.branch ?? (summary.head.detached ? t("app.detached") : "—") }}</span>
        </button>
        <span class="sep"></span>
        <span v-if="summary.identity.email" class="idchip" :title="t('toolbar.identityTitle', { name: summary.identity.name ?? '', email: summary.identity.email, origin: summary.identity.origin ?? '?' })">
          <Icon name="user" />{{ summary.identity.email }}<span class="origin">{{ summary.identity.origin?.replace(/^.*\//, "") }}</span>
        </span>
        <span v-else class="idchip warn" :title="t('toolbar.noIdentityTitle')"><Icon name="warn" />{{ t("toolbar.noIdentity") }}</span>
        <span v-if="summary.status.operation" class="opchip"><Icon name="warn" />{{ t("toolbar.operation", { op: summary.status.operation }) }}</span>
      </template>
      <label class="search" :class="{ on: query }">
        <Icon name="search" />
        <input
          id="search"
          ref="searchInput"
          v-model="query"
          :disabled="!summary"
:placeholder="t('toolbar.searchPlaceholder')"
          spellcheck="false"
          autocomplete="off"
          @input="runSearch"
          @keydown="onSearchKey"
        />
        <span v-if="searchInfo" class="count">
          {{ searchInfo.total ? `${formatNumber(hitIndex + 1, loc)} / ${formatNumber(searchInfo.total, loc)}` : t("toolbar.noResult") }} ·
          {{ searchInfo.ms.toLocaleString(loc, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) }} ms
        </span>
        <kbd v-else>{{ modKey }}F</kbd>
      </label>
      <button class="tool" :title="t('toolbar.palette', { key: `${modKey}K` })" @click="paletteOpen = true"><Icon name="cmd" />{{ modKey }}K</button>
      <button class="tool" :title="t(rainbow ? 'toolbar.rainbowTitle' : 'toolbar.focusTitle')" @click="toggleRainbow">
        <Icon name="palette" />{{ t(rainbow ? "toolbar.rainbow" : "toolbar.focus") }}
      </button>
      <button class="tool" :title="t('toolbar.theme')" @click="cycleTheme"><Icon name="sun" />{{ themeLabel }}</button>
      <div ref="langBox" class="langbox">
        <button class="tool" :title="t('lang.title')" :aria-expanded="langMenu" @click="langMenu = !langMenu"><Icon name="globe" />{{ loc.toUpperCase() }}</button>
        <div v-if="langMenu" class="langmenu" role="menu">
          <button v-for="p in (['', 'fr', 'en'] as LangPref[])" :key="p" class="lmi" :class="{ on: p === langPref }" role="menuitemradio" :aria-checked="p === langPref" @click="chooseLang(p)">
            <Icon name="check" :style="{ visibility: p === langPref ? 'visible' : 'hidden' }" />
            <span v-if="p" :lang="p">{{ langName(p) }}</span>
            <span v-else class="two"><span>{{ t("lang.system") }}</span><small>{{ t("lang.systemDetected", { lang: langName(systemLang()) }) }}</small></span>
          </button>
        </div>
      </div>
    </header>

    <div class="updslot"><UpdateBanner :updater="updater" :has-repo="!!summary" @install="installUpdate" /></div>

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
      <Sidebar :summary="summary" :dirty="wtDirty" @goto="gotoRow" @open="openRepo" />
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
        <p>{{ t("app.tagline") }}</p>
        <button class="btn primary" :disabled="opening || !inTauri" @click="pickFolder"><Icon name="folder" />{{ t("app.openRepo") }} <kbd>{{ modKey }}O</kbd></button>
        <p v-if="!inTauri" class="muted">{{ t("app.browserMode", { file: "public/sample.json" }) }}</p>
        <p v-if="error" class="err">{{ error }}</p>
        <div v-if="activeRecents.length" class="recents">
          <span class="eyebrow">{{ t("app.recents") }}</span>
          <button v-for="(r, i) in activeRecents" :key="r.path" class="recent" @click="openRepo(r.path)">
            <Icon name="repo" /><b>{{ basename(r.path) }}</b><span class="rp">{{ relativeTo(r.path, ws.active) }}</span><span class="when">{{ ago(r.opened_at, loc) }}</span
            ><kbd>{{ modKey }}{{ i + 1 }}</kbd>
          </button>
        </div>
        <p v-else-if="!ws.roots.length" class="muted">{{ t("app.tipRoot") }}</p>
        <p class="muted"><kbd>{{ modKey }}K</kbd> {{ t("app.tipPalette") }}</p>
        <p v-if="opening" class="muted">{{ t("app.opening") }}</p>
      </div>
    </main>
    </div>

    <div v-if="error && summary" class="toast" role="alert">
      <Icon name="warn" /><span>{{ error }}</span><button class="tool" :title="t('app.dismiss')" @click="error = null"><Icon name="x" /></button>
    </div>

    <CommandPalette v-if="paletteOpen" :items="paletteItems" @close="paletteOpen = false" />

    <footer v-if="summary" class="status">
      <span>{{ statusText }}</span>
      <span v-if="summary.trunk_names.length">{{ t("status.trunks", { names: summary.trunk_names.join(", ") }) }}</span>
      <span>{{ t("status.colors", { mode: t(rainbow ? "status.rainbow" : "status.focus") }) }}</span>
      <span class="r">{{ t("status.readOnly", { version: summary.git_version ?? "?" }) }}</span>
    </footer>
  </div>
</template>

<style scoped>
.app {
  height: 100%;
  display: grid;
  grid-template-rows: 44px auto minmax(0, 1fr) auto;
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
.langbox {
  position: relative;
}
.langmenu {
  position: absolute;
  right: 0;
  top: calc(100% + 6px);
  z-index: 30;
  min-width: 200px;
  background: var(--raise);
  border: 1px solid var(--line);
  border-radius: 9px;
  box-shadow: var(--shadow);
  padding: 4px;
}
.lmi {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 0;
  border-radius: 6px;
  background: none;
  cursor: pointer;
  text-align: left;
}
.lmi:hover {
  background: var(--hover);
}
.lmi.on {
  background: var(--sel);
}
.lmi .two {
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.lmi small {
  color: var(--ink-3);
  font-size: 11px;
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
