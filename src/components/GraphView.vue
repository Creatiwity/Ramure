<script setup lang="ts">
// Vue du commit graph : trois zones refs · graph · texte (ux-graph.md D1), lignes virtualisées,
// lanes dessinées sur un canvas limité à la fenêtre visible, largeur de graph constante (D2).
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowReactive, watch } from "vue";
import { api, type Edge, type RepoSummary, type Row, type RowRef } from "../api";
import { readPalette, type Palette } from "../lib/palette";
import { useI18n } from "vue-i18n";
import { dayKey, fullDate, middleEllipsis, parseMessage, relativeDate, splitHighlights, type Locale } from "../lib/format";
import Icon from "./Icon.vue";

const props = defineProps<{
  summary: RepoSummary;
  selected: number | null;
  /** Lignes qui correspondent à la recherche (null : pas de recherche). */
  hits: Set<number> | null;
  query: string;
  conventional: boolean;
  rainbow: boolean;
  /** Incrémenté quand le dépôt a été rechargé : caches à vider. */
  revision: number;
}>();
const emit = defineEmits<{ select: [row: number]; open: [row: number] }>();

const { t, locale } = useI18n();
const loc = computed(() => locale.value as Locale);

const RH = 28;
const OVERSCAN = 12;
const CHUNK = 256;
const AUTHOR_W = 128;
/** En dessous de cette largeur, la colonne auteur passe dans l'infobulle et les refs se resserrent. */
const NARROW = 820;
const DATE_W = 92;
const MAX_VISIBLE_LANES = 12;
const X0 = 12;

const viewport = ref<HTMLDivElement>();
const canvas = ref<HTMLCanvasElement>();
const minimap = ref<HTMLCanvasElement>();
const scrollTop = ref(0);
const height = ref(600);
const width = ref(1000);
const hovered = ref<number | null>(null);

// Caches : lignes et arêtes par tranches de CHUNK lignes.
const rows = shallowReactive(new Map<number, Row>());
const edgeChunks = new Map<number, Edge[]>();
const pending = new Set<number>();
const highlights = shallowReactive(new Map<number, number[]>());
let palette: Palette = readPalette();

const laneW = computed(() => (props.summary.max_lanes > MAX_VISIBLE_LANES ? Math.max(8, Math.floor((MAX_VISIBLE_LANES * 16) / props.summary.max_lanes)) : 16));
const graphW = computed(() => X0 * 2 + Math.max(0, Math.min(props.summary.max_lanes, 40) - 1) * laneW.value + 4);
const total = computed(() => props.summary.rows);
const first = computed(() => Math.max(0, Math.floor(scrollTop.value / RH) - OVERSCAN));
const last = computed(() => Math.min(total.value, Math.ceil((scrollTop.value + height.value) / RH) + OVERSCAN));
const visible = computed(() => {
  const out: number[] = [];
  for (let r = first.value; r < last.value; r++) out.push(r);
  return out;
});
const narrow = computed(() => width.value < NARROW);
const refsW = computed(() => (narrow.value ? 140 : 180));
const columns = computed(() => `${refsW.value}px ${graphW.value}px minmax(0, 1fr) ${narrow.value ? 0 : AUTHOR_W}px ${DATE_W}px 12px`);
const focusChains = computed(() => {
  const s = new Set<number>(props.summary.trunk_chains);
  if (props.summary.head_chain != null) s.add(props.summary.head_chain);
  const h = hovered.value != null ? rows.get(hovered.value) : undefined;
  if (h) s.add(h.chain);
  const sel = props.selected != null ? rows.get(props.selected) : undefined;
  if (sel) s.add(sel.chain);
  return s;
});

function colorFor(color: number, chain: number): string {
  if (props.rainbow || focusChains.value.has(chain)) return palette.vivid[color % 8];
  return palette.muted[color % 8];
}

async function ensure(from: number, to: number) {
  const c0 = Math.floor(from / CHUNK);
  const c1 = Math.floor(Math.max(from, to - 1) / CHUNK);
  const rev = props.revision;
  const jobs: Promise<void>[] = [];
  for (let c = c0; c <= c1; c++) {
    if (edgeChunks.has(c) || pending.has(c)) continue;
    pending.add(c);
    const s = c * CHUNK;
    const e = Math.min(total.value, s + CHUNK);
    jobs.push(
      Promise.all([api.rows(s, e), api.edges(s, e)]).then(([rs, es]) => {
        pending.delete(c);
        if (rev !== props.revision) return;
        for (const r of rs) rows.set(r.row, r);
        edgeChunks.set(c, es);
      }),
    );
  }
  if (jobs.length) {
    await Promise.all(jobs);
    draw();
    loadHighlights();
  }
}

function edgesInWindow(from: number, to: number): Edge[] {
  const seen = new Set<string>();
  const out: Edge[] = [];
  const c0 = Math.floor(from / CHUNK);
  const c1 = Math.floor(Math.max(from, to - 1) / CHUNK);
  for (let c = c0; c <= c1; c++) {
    for (const e of edgeChunks.get(c) ?? []) {
      if (e.from_row >= to || e.to_row < from) continue;
      const k = `${e.from_row}:${e.to_row}:${e.via_lane}`;
      if (!seen.has(k)) {
        seen.add(k);
        out.push(e);
      }
    }
  }
  return out;
}

function draw() {
  const cv = canvas.value;
  if (!cv) return;
  const dpr = window.devicePixelRatio || 1;
  const f = first.value;
  const l = last.value;
  const w = graphW.value;
  const h = Math.max(1, (l - f) * RH);
  if (cv.width !== Math.round(w * dpr) || cv.height !== Math.round(h * dpr)) {
    cv.width = Math.round(w * dpr);
    cv.height = Math.round(h * dpr);
    cv.style.width = `${w}px`;
    cv.style.height = `${h}px`;
  }
  cv.style.transform = `translateY(${f * RH}px)`;
  const ctx = cv.getContext("2d")!;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  const lw = laneW.value;
  const x = (lane: number) => X0 + lane * lw;
  const y = (row: number) => (row - f) * RH + RH / 2;
  const dimmed = (row: number) => props.hits != null && !props.hits.has(row);
  const curve = (x1: number, y1: number, x2: number, y2: number) => ctx.bezierCurveTo(x1, y1 + RH * 0.55, x2, y2 - RH * 0.45, x2, y2);
  const clampY = (v: number) => Math.max(-RH, Math.min(h + RH, v));

  ctx.lineWidth = 2;
  ctx.lineCap = "round";
  for (const e of edgesInWindow(f - 1, l + 1)) {
    const a = e.from_row;
    const b = e.to_row;
    ctx.strokeStyle = colorFor(e.color, e.chain);
    const fromWip = rows.get(a)?.kind === "wip";
    ctx.setLineDash(fromWip ? [3, 3] : []);
    ctx.globalAlpha = dimmed(a) && dimmed(b) ? 0.28 : fromWip ? 0.75 : 1;
    ctx.beginPath();
    ctx.moveTo(x(e.from_lane), clampY(y(a)));
    if (b - a === 1) {
      if (e.from_lane === e.to_lane) ctx.lineTo(x(e.to_lane), y(b));
      else curve(x(e.from_lane), y(a), x(e.to_lane), y(b));
    } else {
      let cy = y(a);
      if (e.via_lane !== e.from_lane) {
        if (y(a + 1) > -RH) curve(x(e.from_lane), y(a), x(e.via_lane), y(a + 1));
        else ctx.moveTo(x(e.via_lane), clampY(y(a + 1)));
        cy = y(a + 1);
      }
      const end = e.to_lane !== e.via_lane ? y(b - 1) : y(b);
      if (end > cy) ctx.lineTo(x(e.via_lane), clampY(end));
      if (e.to_lane !== e.via_lane && y(b - 1) < h + RH) curve(x(e.via_lane), y(b - 1), x(e.to_lane), y(b));
    }
    ctx.stroke();
  }
  ctx.setLineDash([]);

  for (let r = f; r < l; r++) {
    const row = rows.get(r);
    if (!row) continue;
    const cx = x(row.lane);
    const cy = y(r);
    const c = row.kind === "stash" ? palette.neutral : colorFor(row.color, row.chain);
    ctx.globalAlpha = dimmed(r) ? 0.3 : 1;
    const isHead = row.refs.some((x) => x.head) || (props.summary.head.row === r && props.summary.head.detached);
    // Trait de liaison pastille → nœud (D1).
    if (row.refs.length) {
      ctx.strokeStyle = c;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(0, cy);
      ctx.lineTo(cx - (isHead ? 7 : 5), cy);
      ctx.stroke();
      ctx.lineWidth = 2;
    }
    ctx.beginPath();
    if (row.kind === "wip") {
      ctx.setLineDash([2.5, 2]);
      ctx.arc(cx, cy, 5, 0, Math.PI * 2);
      ctx.fillStyle = palette.bg;
      ctx.fill();
      ctx.strokeStyle = c;
      ctx.lineWidth = 1.8;
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.lineWidth = 2;
    } else if (row.kind === "stash") {
      ctx.roundRect(cx - 4.5, cy - 4.5, 9, 9, 2);
      ctx.fillStyle = palette.bg;
      ctx.fill();
      ctx.strokeStyle = c;
      ctx.lineWidth = 1.8;
      ctx.stroke();
      ctx.lineWidth = 2;
    } else if (isHead) {
      ctx.arc(cx, cy, 7, 0, Math.PI * 2);
      ctx.fillStyle = palette.bg;
      ctx.fill();
      ctx.strokeStyle = c;
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx, cy, 3.5, 0, Math.PI * 2);
      ctx.fillStyle = c;
      ctx.fill();
    } else {
      ctx.arc(cx, cy, row.kind === "merge" ? 3.5 : 5, 0, Math.PI * 2);
      ctx.fillStyle = c;
      ctx.fill();
      ctx.strokeStyle = palette.bg;
      ctx.stroke();
    }
  }
  ctx.globalAlpha = 1;
  drawMinimap();
}

function drawMinimap() {
  const cv = minimap.value;
  if (!cv) return;
  const dpr = window.devicePixelRatio || 1;
  const h = height.value;
  const w = 12;
  cv.width = w * dpr;
  cv.height = h * dpr;
  cv.style.height = `${h}px`;
  const ctx = cv.getContext("2d")!;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  const n = Math.max(1, total.value);
  const yOf = (row: number) => (row / n) * h;
  ctx.fillStyle = palette.line;
  const thumbH = Math.max(12, (height.value / (n * RH)) * h);
  ctx.globalAlpha = 0.9;
  ctx.fillRect(1, (scrollTop.value / (n * RH)) * h, w - 2, thumbH);
  ctx.globalAlpha = 1;
  const mark = (row: number, color: string, hh = 2) => {
    ctx.fillStyle = color;
    ctx.fillRect(2, Math.min(h - hh, yOf(row)), w - 4, hh);
  };
  if (props.hits) for (const r of props.hits) mark(r, palette.mark, 2);
  for (const r of props.summary.refs) {
    if (r.kind === "tag") mark(r.row, palette.muted[0], 1);
    else if (r.kind === "local") mark(r.row, r.head ? palette.vivid[3] : palette.vivid[0], r.head ? 3 : 2);
  }
}

function onScroll() {
  scrollTop.value = viewport.value!.scrollTop;
}

function measure() {
  const el = viewport.value;
  if (!el) return;
  height.value = el.clientHeight;
  width.value = el.clientWidth;
}

watch([first, last], ([f, l]) => {
  ensure(f, l);
  draw();
  loadHighlights();
});
watch(() => [props.hits, props.rainbow, props.selected, hovered.value], () => draw());
watch(
  () => props.revision,
  () => {
    rows.clear();
    edgeChunks.clear();
    pending.clear();
    highlights.clear();
    ensure(first.value, last.value);
  },
);

// Surlignage : seulement pour les lignes visibles qui correspondent.
let hlToken = 0;
async function loadHighlights() {
  const token = ++hlToken;
  if (!props.hits || !props.query.trim()) {
    highlights.clear();
    return;
  }
  const want = visible.value.filter((r) => props.hits!.has(r) && !highlights.has(r));
  if (!want.length) return;
  const res = await api.highlights(props.query, want);
  if (token !== hlToken) return;
  want.forEach((r, i) => highlights.set(r, res[i]));
}
watch(
  () => [props.query, props.hits],
  () => {
    highlights.clear();
    loadHighlights();
  },
);

let ro: ResizeObserver | null = null;
const media = window.matchMedia("(prefers-color-scheme: dark)");
const themeObserver = new MutationObserver(() => refreshPalette());
function refreshPalette() {
  palette = readPalette();
  draw();
}
onMounted(() => {
  measure();
  ro = new ResizeObserver(() => {
    measure();
    draw();
  });
  ro.observe(viewport.value!);
  media.addEventListener("change", refreshPalette);
  themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  ensure(first.value, last.value);
});
onBeforeUnmount(() => {
  ro?.disconnect();
  media.removeEventListener("change", refreshPalette);
  themeObserver.disconnect();
});

/** Fait défiler pour rendre une ligne visible (centrée si elle est loin). */
function reveal(row: number, center = false) {
  const el = viewport.value;
  if (!el) return;
  const top = row * RH;
  if (center || top < el.scrollTop + RH || top > el.scrollTop + el.clientHeight - 2 * RH) {
    el.scrollTop = center ? top - el.clientHeight / 2 + RH / 2 : top < el.scrollTop + RH ? top - RH : top - el.clientHeight + 2 * RH;
  }
}
defineExpose({ reveal, focus: () => viewport.value?.focus() });

function onKey(e: KeyboardEvent) {
  const cur = props.selected ?? -1;
  const page = Math.max(1, Math.floor(height.value / RH) - 2);
  let next: number | null = null;
  if (e.key === "ArrowDown" || e.key === "j") next = cur + 1;
  else if (e.key === "ArrowUp" || e.key === "k") next = cur - 1;
  else if (e.key === "PageDown") next = cur + page;
  else if (e.key === "PageUp") next = cur - page;
  else if (e.key === "Home") next = 0;
  else if (e.key === "End") next = total.value - 1;
  else if (e.key === "h" && props.summary.head.row != null) next = props.summary.head.row;
  else if (e.key === "Enter" && props.selected != null) {
    emit("open", props.selected);
    return;
  } else return;
  e.preventDefault();
  next = Math.max(0, Math.min(total.value - 1, next));
  emit("select", next);
  reveal(next, e.key === "h" || e.key === "Home" || e.key === "End");
}

function onMinimap(e: MouseEvent) {
  const cv = minimap.value!;
  const rect = cv.getBoundingClientRect();
  const row = Math.floor(((e.clientY - rect.top) / rect.height) * total.value);
  reveal(Math.max(0, Math.min(total.value - 1, row)), true);
}

// --- Affichage d'une ligne -------------------------------------------------

const PRIO: Record<string, number> = { head: 0, local: 1, remote: 2, tag: 3, stash: 4 };
function sortedRefs(r: Row): RowRef[] {
  return [...r.refs].sort((a, b) => PRIO[a.head ? "head" : a.kind] - PRIO[b.head ? "head" : b.kind]);
}
function refIcons(ref: RowRef): string[] {
  if (ref.head) return ["check"];
  if (ref.kind === "remote") return ["cloud"];
  if (ref.kind === "tag") return ["tag"];
  if (ref.kind === "stash") return ["box"];
  return ref.remote ? ["laptop", "cloud"] : ["laptop"];
}
function refTitle(r: Row): string {
  return sortedRefs(r)
    .map((x) => (x.remote ? `${x.name} = ${x.remote}` : x.name) + (x.head ? ` (${t("graph.head")})` : ""))
    .join("\n");
}
function pillColor(r: Row): string {
  return r.kind === "stash" ? palette.neutral : colorFor(r.color, r.chain);
}
const topRow = computed(() => Math.floor(scrollTop.value / RH));
/** Date au changement de jour (D6), et toujours sur la première ligne visible pour garder le repère. */
function showDate(r: Row): boolean {
  if (r.row <= topRow.value) return true;
  const prev = rows.get(r.row - 1);
  return !prev || prev.kind === "wip" || dayKey(prev.time) !== dayKey(r.time);
}
function repeatedAuthor(r: Row): boolean {
  const prev = rows.get(r.row - 1);
  return !!prev && prev.author === r.author && prev.kind !== "wip";
}
function wipLabel(): string {
  const s = props.summary.status;
  const changed = s.unstaged + s.untracked + s.conflicted;
  const parts = [];
  if (changed) parts.push(t("graph.wipChanged", changed));
  if (s.staged) parts.push(t("graph.wipStaged", s.staged));
  if (s.conflicted) parts.push(t("graph.wipConflicted", s.conflicted));
  return parts.join(" · ") || t("graph.wipNone");
}

function pieces(r: Row) {
  const idx = highlights.get(r.row) ?? [];
  if (!r.summary && r.kind !== "wip") return { type: null, scope: [], merged: null, pr: null, parts: [{ text: t("graph.unreadable"), hit: false }] };
  if (!props.conventional || r.kind === "stash") return { type: r.kind === "stash" ? "stash" : null, scope: [], merged: null, pr: null, parts: splitHighlights(r.summary, idx) };
  const m = parseMessage(r.summary);
  if (m.merged) return { type: "merge", scope: [], merged: m.merged, pr: m.pr, parts: [] };
  return { type: m.type, scope: m.scope ? splitHighlights(m.scope, idx, m.scopeOffset) : [], merged: null, pr: null, parts: splitHighlights(m.subject, idx, m.offset) };
}

interface ViewRow {
  r: number;
  row: Row;
  top: RowRef | null;
  icons: string[];
  more: number;
  title: string;
  color: string;
  view: ReturnType<typeof pieces>;
  date: string;
  repeated: boolean;
}

const viewRows = computed<(ViewRow | { r: number; row: null })[]>(() =>
  visible.value.map((r) => {
    const row = rows.get(r);
    if (!row) return { r, row: null };
    const refs = sortedRefs(row);
    return {
      r,
      row,
      top: refs[0] ?? null,
      icons: refs[0] ? refIcons(refs[0]) : [],
      more: Math.max(0, refs.length - 1),
      title: refTitle(row),
      color: pillColor(row),
      view: pieces(row),
      date: row.kind !== "wip" && showDate(row) ? relativeDate(row.time, loc.value) : "",
      repeated: repeatedAuthor(row),
    };
  }),
);

async function selectRow(row: number) {
  emit("select", row);
  await nextTick();
  viewport.value?.focus();
}
</script>

<template>
  <div class="graph" :style="{ '--cols': columns }">
    <div class="ghead">
      <span>{{ t("graph.refs") }}</span><span></span><span>{{ t("graph.message") }}</span><span>{{ narrow ? "" : t("graph.author") }}</span><span>{{ t("graph.date") }}</span><span></span>
    </div>
    <div ref="viewport" class="viewport" tabindex="0" @scroll.passive="onScroll" @keydown="onKey" @mouseleave="hovered = null">
      <div class="spacer" :style="{ height: total * RH + 'px' }">
        <canvas ref="canvas" class="lanes" :style="{ left: refsW + 'px' }" />
        <template v-for="v in viewRows" :key="v.r">
          <div
            v-if="v.row"
            class="row"
            :class="{ sel: selected === v.r, dim: hits && !hits.has(v.r), merge: v.row.kind === 'merge' }"
            :style="{ top: v.r * RH + 'px' }"
            @mousedown="selectRow(v.r)"
            @dblclick="emit('open', v.r)"
            @mouseenter="hovered = v.r"
          >
            <span class="refs" :title="v.title">
              <template v-if="v.top">
                <span v-if="v.more" class="more">+{{ v.more }}</span>
                <span class="pill" :class="{ head: v.top.head, tag: v.top.kind === 'tag' }" :style="{ '--c': v.color }">
                  <Icon v-for="ic in v.icons" :key="ic" :name="ic as any" />
                  <span>{{ middleEllipsis(v.top.name, narrow ? 16 : 22) }}</span>
                </span>
              </template>
            </span>
            <span></span>
            <span class="msg" :title="v.row.kind === 'wip' ? '' : v.row.summary">
              <template v-if="v.row.kind === 'wip'">
                <span v-if="conventional" class="ctype"><Icon name="pencil" /></span>
                <Icon v-else name="pencil" />
                <span class="txt wiptxt">{{ wipLabel() }}</span>
              </template>
              <template v-else>
                <span v-if="conventional" class="ctype">{{ v.view.type ?? "" }}</span>
                <span v-if="v.view.merged" class="txt mg">
                  ⤙ <b>{{ v.view.merged }}</b><span v-if="v.view.pr" class="pr">#{{ v.view.pr }}</span>
                </span>
                <span v-else class="txt"
                  ><span v-if="v.view.scope.length" class="scope"><template v-for="(p, i) in v.view.scope" :key="i"><mark v-if="p.hit">{{ p.text }}</mark><template v-else>{{ p.text }}</template></template></span
                  ><template v-for="(p, i) in v.view.parts" :key="i"><mark v-if="p.hit">{{ p.text }}</mark><template v-else>{{ p.text }}</template></template></span
                >
              </template>
            </span>
            <span class="who" :class="{ rep: v.repeated, gone: narrow }">{{ narrow ? "" : v.row.author }}</span>
            <span class="when" :title="v.row.time ? (narrow ? `${v.row.author} · ` : '') + fullDate(v.row.time, loc) : ''">{{ v.date }}</span>
            <span></span>
          </div>
          <div v-else class="row placeholder" :style="{ top: v.r * RH + 'px' }"><span></span><span></span><span class="msg"><i></i></span></div>
        </template>
      </div>
    </div>
    <canvas ref="minimap" class="minimap" :title="t('graph.minimap')" @mousedown="onMinimap" />
  </div>
</template>

<style scoped>
.graph {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  background: var(--bg);
}
.ghead {
  display: grid;
  grid-template-columns: var(--cols);
  height: 26px;
  align-items: center;
  border-bottom: 1px solid var(--line);
  color: var(--ink-3);
  font-size: 10.5px;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  font-weight: 600;
  flex: none;
}
.ghead span {
  padding: 0 10px;
}
.viewport {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  outline: none;
  scrollbar-width: none;
}
.viewport::-webkit-scrollbar {
  display: none;
}
.spacer {
  position: relative;
  min-width: 100%;
}
.lanes {
  position: absolute;
  top: 0;
  pointer-events: none;
  z-index: 2;
}
.row {
  position: absolute;
  left: 0;
  right: 0;
  height: 28px;
  display: grid;
  grid-template-columns: var(--cols);
  align-items: center;
  cursor: default;
}
.row:hover {
  background: var(--hover);
}
.row.sel {
  background: var(--sel);
}
.row > span {
  padding: 0 10px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  min-width: 0;
}
.row > span.refs {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 4px;
  padding: 0 0 0 8px;
}
.row > span.gone {
  padding: 0;
}
.refs .pill {
  flex: 0 1 auto;
  min-width: 0;
}
.more {
  font: 600 10.5px var(--f-mono);
  color: var(--ink-3);
  flex: none;
}
.msg {
  display: flex;
  align-items: center;
  gap: 6px;
}
.msg .txt {
  overflow: hidden;
  text-overflow: ellipsis;
}
.ctype {
  flex: none;
  width: 60px;
  font: 500 11px var(--f-mono);
  color: var(--ink-3);
  display: inline-flex;
}
.scope {
  color: var(--ink-3);
  margin-right: 6px;
}
.txt.mg {
  color: var(--ink-3);
}
.mg b {
  font-weight: 500;
  color: var(--ink-2);
}
.pr {
  font: 11px var(--f-mono);
  color: var(--ink-3);
  margin-left: 8px;
}
.wiptxt {
  color: var(--ink-3);
  font-style: italic;
}
.who {
  color: var(--ink-2);
}
.who.rep {
  color: var(--ink-3);
  opacity: 0.6;
}
.when {
  color: var(--ink-3);
  font-variant-numeric: tabular-nums;
}
.row.dim > span:not(.refs) {
  opacity: 0.35;
}
.placeholder .msg i {
  display: block;
  width: 40%;
  height: 8px;
  border-radius: 4px;
  background: var(--hover);
}
.minimap {
  position: absolute;
  right: 0;
  top: 26px;
  width: 12px;
  border-left: 1px solid var(--line);
  background: var(--panel);
  cursor: pointer;
  z-index: 3;
}
</style>
