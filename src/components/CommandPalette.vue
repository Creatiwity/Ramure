<script setup lang="ts">
// Palette de commandes (⌘K) : dépôts récents de tous les contextes, dépôts du contexte actif,
// branches et tags du dépôt ouvert, actions. Même clavier partout : ↑↓, ↵, Échap.
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { rank, type FuzzyMatch } from "../lib/fuzzy";
import { splitHighlights } from "../lib/format";
import { useI18n } from "vue-i18n";
import Icon from "./Icon.vue";

export interface PaletteItem {
  id: string;
  group: string;
  label: string;
  /** Texte secondaire (chemin, date…). */
  detail?: string;
  icon: string;
  hint?: string;
  /** Texte utilisé pour la correspondance (par défaut : le libellé). */
  search?: string;
  run: () => void;
}

const props = defineProps<{ items: PaletteItem[]; placeholder?: string }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const query = ref("");
const active = ref(0);
const input = ref<HTMLInputElement>();
const list = ref<HTMLDivElement>();

const results = computed(() => {
  const ranked = rank(props.items, query.value, (i) => i.search ?? i.label);
  // Sans requête : l'ordre des groupes fourni ; avec requête : le meilleur score d'abord, en
  // gardant les éléments d'un même groupe ensemble derrière leur meilleur représentant.
  if (!query.value.trim()) return ranked.slice(0, 60);
  const groups = new Map<string, { item: PaletteItem; match: FuzzyMatch }[]>();
  for (const r of ranked) groups.set(r.item.group, [...(groups.get(r.item.group) ?? []), r]);
  return [...groups.values()].flatMap((g) => g.slice(0, 12)).slice(0, 60);
});
watch(query, () => (active.value = 0));

function labelParts(r: { item: PaletteItem; match: FuzzyMatch }) {
  // Les positions sont calculées sur `search` ; on ne les applique que si elles portent sur le libellé.
  return r.item.search && r.item.search !== r.item.label ? [{ text: r.item.label, hit: false }] : splitHighlights(r.item.label, r.match.indices);
}

function run(i: number) {
  const r = results.value[i];
  if (!r) return;
  emit("close");
  r.item.run();
}
function onKey(e: KeyboardEvent) {
  if (e.key === "ArrowDown") active.value = Math.min(results.value.length - 1, active.value + 1);
  else if (e.key === "ArrowUp") active.value = Math.max(0, active.value - 1);
  else if (e.key === "Enter") run(active.value);
  else if (e.key === "Escape") emit("close");
  else return;
  e.preventDefault();
  nextTick(() => list.value?.querySelector(".mi.hot")?.scrollIntoView({ block: "nearest" }));
}
onMounted(() => input.value?.focus());
</script>

<template>
  <div class="veil" @mousedown.self="emit('close')">
    <div class="palette" role="dialog" :aria-label="t('palette.title')">
      <label class="in">
        <Icon name="search" />
        <input id="palette" ref="input" v-model="query" :placeholder="placeholder ?? t('palette.placeholder')" spellcheck="false" autocomplete="off" @keydown="onKey" />
      </label>
      <div ref="list" class="res">
        <template v-for="(r, i) in results" :key="r.item.id">
          <div v-if="i === 0 || results[i - 1].item.group !== r.item.group" class="mlabel">{{ r.item.group }}</div>
          <button class="mi" :class="{ hot: i === active }" @mousemove="active = i" @click="run(i)">
            <Icon :name="r.item.icon as any" />
            <span class="two">
              <span class="lbl"><template v-for="(p, k) in labelParts(r)" :key="k"><b v-if="p.hit">{{ p.text }}</b><template v-else>{{ p.text }}</template></template></span>
              <span v-if="r.item.detail" class="dt">{{ r.item.detail }}</span>
            </span>
            <kbd v-if="r.item.hint">{{ r.item.hint }}</kbd>
          </button>
        </template>
        <div v-if="!results.length" class="none">{{ t("palette.none") }}</div>
      </div>
      <footer>
        <span><kbd>↑</kbd><kbd>↓</kbd> {{ t("palette.navigate") }}</span><span><kbd>↵</kbd> {{ t("palette.open") }}</span><span><kbd>{{ t("palette.esc") }}</kbd> {{ t("palette.close") }}</span>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.veil {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: color-mix(in srgb, var(--bg) 40%, transparent);
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding-top: 72px;
}
.palette {
  width: min(620px, 92vw);
  max-height: min(560px, 76vh);
  display: flex;
  flex-direction: column;
  background: var(--raise);
  border: 1px solid var(--line);
  border-radius: 12px;
  box-shadow: var(--shadow);
  overflow: hidden;
}
.in {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--line);
  color: var(--ink-3);
}
.in input {
  flex: 1;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--ink);
  font: 15px var(--f-ui);
}
.res {
  overflow-y: auto;
  padding: 6px;
}
.mlabel {
  padding: 8px 9px 3px;
  font-size: 10.5px;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--ink-3);
  font-weight: 600;
}
.mi {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 9px;
  border: 0;
  border-radius: 6px;
  background: none;
  text-align: left;
  cursor: pointer;
  min-width: 0;
}
.mi.hot {
  background: var(--accent);
  color: var(--accent-ink);
}
.mi.hot .dt,
.mi.hot kbd {
  color: inherit;
  opacity: 0.8;
}
.mi.hot kbd {
  background: transparent;
  border-color: currentColor;
}
.two {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
  flex: 1;
}
.lbl,
.dt {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.lbl b {
  font-weight: 700;
}
.dt {
  font-size: 11px;
  color: var(--ink-3);
}
.none {
  padding: 14px;
  color: var(--ink-3);
}
footer {
  display: flex;
  gap: 14px;
  padding: 8px 14px;
  border-top: 1px solid var(--line);
  background: var(--panel);
  color: var(--ink-3);
  font-size: 11.5px;
}
footer span {
  display: flex;
  gap: 5px;
  align-items: center;
}
</style>
