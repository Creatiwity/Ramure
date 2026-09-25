<script setup lang="ts">
// Panneau des espaces de travail : un dossier racine actif (le contexte), ses dépôts récents
// avec leur date d'ouverture et un raccourci ⌘1…⌘9, et l'arbre des dépôts trouvés dedans, avec
// les dossiers intermédiaires fusionnés (« clients / acme / apps »), comme VS Code.
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ScanResult, TreeNode, WorkspaceStore } from "../api";
import { useI18n } from "vue-i18n";
import { ago, basename, relativeTo, type Locale } from "../lib/format";
import { rank } from "../lib/fuzzy";
import Icon from "./Icon.vue";

const props = defineProps<{
  store: WorkspaceStore;
  scan: ScanResult | null;
  scanning: boolean;
  current: string | null;
  canPick: boolean;
}>();
const emit = defineEmits<{
  open: [path: string];
  add: [];
  remove: [root: string];
  activate: [root: string];
  rescan: [];
  forget: [repo: string];
  collapse: [];
}>();

const { t, locale } = useI18n();
const loc = computed(() => locale.value as Locale);
const menuOpen = ref(false);
const picker = ref<HTMLDivElement>();
// Le menu des dossiers racines se ferme au clic extérieur et avec Échap.
function onDocDown(e: MouseEvent) {
  if (menuOpen.value && picker.value && !picker.value.contains(e.target as Node)) menuOpen.value = false;
}
function onDocKey(e: KeyboardEvent) {
  if (e.key === "Escape") menuOpen.value = false;
}
onMounted(() => {
  document.addEventListener("mousedown", onDocDown);
  document.addEventListener("keydown", onDocKey);
});
onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocDown);
  document.removeEventListener("keydown", onDocKey);
});
const confirmRemove = ref<string | null>(null);
const filter = ref("");
const expanded = ref(new Set<string>());

const active = computed(() => props.store.roots.find((r) => r.path === props.store.active) ?? null);
const recents = computed(() => (active.value ? active.value.recent : props.store.recent_outside).slice(0, 9));

function key(root: string) {
  return `ramure.ws.expanded:${root}`;
}
function loadExpanded(root: string | null) {
  let paths: string[] = [];
  try {
    paths = root ? JSON.parse(localStorage.getItem(key(root)) ?? "null") : [];
  } catch {
    /* préférence illisible : état par défaut */
  }
  // Par défaut : premier niveau déplié.
  expanded.value = new Set(paths ?? (props.scan?.tree.filter((n) => n.kind === "dir").map((n) => n.path) ?? []));
  revealCurrent();
}
function saveExpanded() {
  if (!props.store.active) return;
  try {
    localStorage.setItem(key(props.store.active), JSON.stringify([...expanded.value]));
  } catch {
    /* non mémorisé */
  }
}
/** Déplie les dossiers qui mènent au dépôt ouvert. */
function revealCurrent() {
  if (!props.current || !props.scan) return;
  const walk = (nodes: TreeNode[], trail: string[]): boolean => {
    for (const n of nodes) {
      if (n.kind === "repo" && n.path === props.current) {
        trail.forEach((p) => expanded.value.add(p));
        return true;
      }
      if (n.kind === "dir" && walk(n.children, [...trail, n.path])) return true;
    }
    return false;
  };
  walk(props.scan.tree, []);
}
watch(() => props.scan, () => loadExpanded(props.store.active), { immediate: true });
watch(() => props.current, revealCurrent);

function toggle(n: TreeNode) {
  if (expanded.value.has(n.path)) expanded.value.delete(n.path);
  else expanded.value.add(n.path);
  expanded.value = new Set(expanded.value);
  saveExpanded();
}

interface Line {
  node: TreeNode;
  depth: number;
}
/** Arbre aplati en lignes visibles. */
const lines = computed<Line[]>(() => {
  const out: Line[] = [];
  const walk = (nodes: TreeNode[], depth: number) => {
    for (const n of nodes) {
      out.push({ node: n, depth });
      if (n.kind === "dir" && expanded.value.has(n.path)) walk(n.children, depth + 1);
    }
  };
  walk(props.scan?.tree ?? [], 0);
  return out;
});

/** Avec un filtre : liste plate des dépôts, classés par correspondance floue sur le chemin. */
const allRepos = computed(() => {
  const out: TreeNode[] = [];
  const walk = (nodes: TreeNode[]) => nodes.forEach((n) => (n.kind === "repo" ? out.push(n) : walk(n.children)));
  walk(props.scan?.tree ?? []);
  return out;
});
const filtered = computed(() => rank(allRepos.value, filter.value, (n) => relativeTo(n.path, props.scan?.root ?? null)).slice(0, 200));

function pickRoot(path: string) {
  menuOpen.value = false;
  confirmRemove.value = null;
  emit("activate", path);
}
function onFilterKey(e: KeyboardEvent) {
  if (e.key === "Enter" && filtered.value[0]) emit("open", filtered.value[0].item.path);
  if (e.key === "Escape") filter.value = "";
}
const mod = navigator.platform.includes("Mac") ? "⌘" : "Ctrl+";
/** Dossier parent relatif à la racine (« clients/acme/apps »), vide si le dépôt est à la racine. */
function parentOf(path: string): string {
  const rel = relativeTo(path, active.value?.path ?? null);
  const i = rel.lastIndexOf("/");
  return i > 0 ? rel.slice(0, i) : "";
}

function nodeTitle(n: TreeNode): string {
  return n.worktree_of ? t("ws.worktreeOf", { repo: n.worktree_of, path: n.path }) : n.path;
}
</script>

<template>
  <aside class="ws">
    <header>
      <span class="eyebrow">{{ t("ws.title") }}</span>
      <button class="icon" :title="t('ws.collapse', { key: `${mod}⇧E` })" @click="emit('collapse')"><Icon name="panel" /></button>
    </header>

    <div ref="picker" class="root-picker">
      <button class="root" :class="{ open: menuOpen }" :title="active?.path ?? t('ws.noRoot')" @click="menuOpen = !menuOpen">
        <Icon name="folder" />
        <span class="rname">{{ active ? basename(active.path) : t("ws.noRoot") }}</span>
        <Icon name="chev" class="chev" />
      </button>
      <div v-if="menuOpen" class="menu" @mouseleave="confirmRemove = null">
        <div v-for="r in store.roots" :key="r.path" class="mi" :class="{ cur: r.path === store.active }">
          <button class="pick" :title="r.path" @click="pickRoot(r.path)">
            <Icon :name="r.path === store.active ? 'check' : 'folder'" />
            <span class="two"><b>{{ basename(r.path) }}</b><span>{{ r.path }}</span></span>
          </button>
          <button v-if="confirmRemove !== r.path" class="icon del" :title="t('ws.removeTitle')" @click="confirmRemove = r.path">
            <Icon name="trash" />
          </button>
          <button v-else class="confirm" @click="(emit('remove', r.path), (confirmRemove = null))">{{ t("ws.remove") }}</button>
        </div>
        <div v-if="store.roots.length" class="sep"></div>
        <button class="mi add" :disabled="!canPick" @click="((menuOpen = false), emit('add'))"><Icon name="plus" />{{ t("ws.addRoot") }}</button>
      </div>
    </div>

    <div v-if="!store.roots.length" class="empty">
      <p>{{ t("ws.empty", { example: "~/code" }) }}</p>
      <button class="btn primary" :disabled="!canPick" @click="emit('add')"><Icon name="plus" />{{ t("ws.addRoot") }}</button>
    </div>

    <section v-if="recents.length" class="sec">
      <div class="sh">
        <span class="eyebrow">{{ t("ws.recents") }}</span>
      </div>
      <div v-for="(r, i) in recents" :key="r.path" class="recent" :class="{ cur: r.path === current }">
        <button class="go" :title="r.path" @click="emit('open', r.path)">
          <Icon name="repo" />
          <span class="two">
            <b>{{ basename(r.path) }}</b>
            <span>{{ parentOf(r.path) ? `${parentOf(r.path)} · ` : "" }}{{ ago(r.opened_at, loc) }}</span>
          </span>
          <kbd>{{ mod }}{{ i + 1 }}</kbd>
        </button>
        <button class="icon forget" :title="t('ws.forget')" @click="emit('forget', r.path)"><Icon name="x" /></button>
      </div>
    </section>

    <section v-if="active" class="sec grow">
      <div class="sh">
        <span class="eyebrow">{{ t("ws.repos") }}</span>
        <span class="meta">{{ scanning ? t("ws.scanning") : scan ? `${scan.repos}${scan.truncated ? "+" : ""}` : "" }}</span>
        <button class="icon" :title="t('ws.rescan')" :disabled="scanning" @click="emit('rescan')"><Icon name="refresh" /></button>
      </div>
      <label class="filter">
        <Icon name="search" />
        <input id="ws-filter" v-model="filter" :placeholder="t('ws.filter')" spellcheck="false" @keydown="onFilterKey" />
      </label>
      <div class="tree">
        <template v-if="filter.trim()">
          <button v-for="f in filtered" :key="f.item.path" class="node repo" :class="{ cur: f.item.path === current }" :title="nodeTitle(f.item)" @click="emit('open', f.item.path)">
            <Icon :name="f.item.worktree_of ? 'wt' : 'repo'" />
            <span class="nm">{{ relativeTo(f.item.path, scan?.root ?? null) }}</span>
            <span v-if="f.item.branch" class="br">{{ f.item.branch }}</span>
          </button>
          <div v-if="!filtered.length" class="none">{{ t("ws.noMatch") }}</div>
        </template>
        <template v-else>
          <button
            v-for="l in lines"
            :key="l.node.path"
            class="node"
            :class="[l.node.kind, { cur: l.node.path === current }]"
            :style="{ paddingLeft: 10 + l.depth * 14 + 'px' }"
            :title="nodeTitle(l.node)"
            @click="l.node.kind === 'dir' ? toggle(l.node) : emit('open', l.node.path)"
          >
            <Icon v-if="l.node.kind === 'dir'" name="chev" class="tw" :class="{ open: expanded.has(l.node.path) }" />
            <span v-else class="tw-space"></span>
            <Icon :name="l.node.kind === 'dir' ? 'folder' : l.node.worktree_of ? 'wt' : 'repo'" />
            <span class="nm">{{ l.node.name }}</span>
            <span v-if="l.node.branch" class="br">{{ l.node.branch }}</span>
          </button>
          <div v-if="scan && !scan.tree.length" class="none">{{ t("ws.noRepos") }}</div>
          <div v-if="scan?.truncated" class="none">{{ t("ws.truncated", { n: scan.repos }) }}</div>
        </template>
      </div>
    </section>
  </aside>
</template>

<style scoped>
.ws {
  background: var(--panel);
  border-right: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  min-height: 0;
  min-width: 0;
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 8px 4px 12px;
}
.icon {
  border: 0;
  background: none;
  color: var(--ink-3);
  cursor: pointer;
  padding: 3px;
  border-radius: 5px;
  display: inline-flex;
}
.icon:hover {
  background: var(--hover);
  color: var(--ink);
}
.root-picker {
  position: relative;
  padding: 0 8px 6px;
}
.root {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 8px;
  border-radius: 7px;
  border: 1px solid var(--line);
  background: var(--bg);
  cursor: pointer;
  font-weight: 600;
  min-width: 0;
}
.rname {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  text-align: left;
}
.chev {
  transform: rotate(90deg);
  color: var(--ink-3);
}
.root.open .chev {
  transform: rotate(-90deg);
}
.menu {
  position: absolute;
  left: 8px;
  right: 8px;
  top: calc(100% - 2px);
  z-index: 20;
  background: var(--raise);
  border: 1px solid var(--line);
  border-radius: 9px;
  box-shadow: var(--shadow);
  padding: 4px;
}
.mi {
  display: flex;
  align-items: center;
  gap: 4px;
  border-radius: 6px;
  width: 100%;
}
.mi.cur {
  background: var(--sel);
}
.mi:not(.cur):hover {
  background: var(--hover);
}
.pick,
.mi.add {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 0;
  background: none;
  cursor: pointer;
  text-align: left;
  min-width: 0;
}
.two {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 1px;
}
.two b {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.two span {
  font-size: 11px;
  color: var(--ink-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.del {
  margin-right: 4px;
}
.confirm {
  margin-right: 4px;
  border: 0;
  border-radius: 5px;
  padding: 3px 8px;
  background: var(--del);
  color: #fff;
  cursor: pointer;
  font-size: 11.5px;
}
.sep {
  height: 1px;
  background: var(--line);
  margin: 4px 6px;
}
.empty {
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
  color: var(--ink-2);
  line-height: 1.45;
}
.empty p {
  margin: 0;
}
.sec {
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding-bottom: 6px;
}
.sec.grow {
  flex: 1;
}
.sh {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px 3px 12px;
}
.sh .meta {
  margin-left: auto;
  font: 10.5px var(--f-mono);
  color: var(--ink-3);
}
.recent {
  display: flex;
  align-items: center;
  position: relative;
}
.recent.cur {
  background: var(--sel);
}
.recent:not(.cur):hover {
  background: var(--hover);
}
.go {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 6px 5px 12px;
  border: 0;
  background: none;
  cursor: pointer;
  text-align: left;
  min-width: 0;
}
.go .two {
  flex: 1;
}
.go kbd {
  font-size: 10px;
  padding: 1px 4px;
}
.forget {
  visibility: hidden;
  margin-right: 6px;
}
.recent:hover .forget {
  visibility: visible;
}
.recent:hover kbd {
  display: none;
}
.filter {
  margin: 2px 8px 6px;
  height: 26px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: var(--bg);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px;
  color: var(--ink-3);
}
.filter input {
  border: 0;
  background: transparent;
  outline: none;
  color: var(--ink);
  width: 100%;
  font: inherit;
}
.tree {
  overflow-y: auto;
  min-height: 0;
  flex: 1;
}
.node {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border: 0;
  background: none;
  cursor: pointer;
  text-align: left;
  color: var(--ink-2);
  min-width: 0;
}
.node:hover {
  background: var(--hover);
  color: var(--ink);
}
.node.repo {
  color: var(--ink);
}
.node.cur {
  background: var(--sel);
  font-weight: 600;
}
.node .tw {
  width: 11px;
  height: 11px;
  color: var(--ink-3);
  transition: transform 0.12s;
}
.node .tw.open {
  transform: rotate(90deg);
}
.tw-space {
  width: 11px;
  flex: none;
}
.nm {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.br {
  margin-left: auto;
  font: 10.5px var(--f-mono);
  color: var(--ink-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 45%;
  padding-left: 6px;
}
.none {
  padding: 6px 12px;
  color: var(--ink-3);
  font-style: italic;
}
</style>
