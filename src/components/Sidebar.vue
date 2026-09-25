<script setup lang="ts">
import { computed, ref } from "vue";
import type { RefInfo, RepoSummary, Worktree } from "../api";
import { useI18n } from "vue-i18n";
import Icon from "./Icon.vue";

const { t } = useI18n();

const props = defineProps<{ summary: RepoSummary; dirty?: Record<string, boolean | null> }>();
const emit = defineEmits<{ goto: [row: number]; open: [path: string] }>();

/** Section affichée seulement s'il existe d'autres worktrees que le principal. */
const worktrees = computed<Worktree[]>(() => {
  const list = props.summary.worktrees ?? [];
  if (list.length < 2) return [];
  const f = filter.value.trim().toLowerCase();
  return list.filter((w) => !f || `${w.name} ${w.branch ?? ""}`.toLowerCase().includes(f));
});
const wtName = (path: string) => props.summary.worktrees?.find((w) => w.path === path)?.name ?? path.replace(/^.*[\\/]/, "");
function openWorktree(w: Worktree) {
  if (!w.current && !w.prunable) emit("open", w.path);
}

const filter = ref("");
const collapsed = ref<Record<string, boolean>>({});

interface Group {
  key: string;
  label: string;
  icon: "laptop" | "cloud" | "tag" | "box";
  items: RefInfo[];
}

const groups = computed<Group[]>(() => {
  const f = filter.value.trim().toLowerCase();
  const keep = (r: RefInfo) => !f || r.name.toLowerCase().includes(f);
  const refs = props.summary.refs.filter(keep);
  const byName = (a: RefInfo, b: RefInfo) => (a.head ? -1 : b.head ? 1 : a.name.localeCompare(b.name));
  const out: Group[] = [{ key: "local", label: t("sidebar.local"), icon: "laptop", items: refs.filter((r) => r.kind === "local").sort(byName) }];
  const remotes = new Map<string, RefInfo[]>();
  for (const r of refs.filter((r) => r.kind === "remote")) {
    const name = r.name.split("/")[0];
    remotes.set(name, [...(remotes.get(name) ?? []), r]);
  }
  for (const [name, items] of [...remotes].sort()) out.push({ key: `remote:${name}`, label: name, icon: "cloud", items: items.sort(byName) });
  out.push({ key: "tags", label: t("sidebar.tags"), icon: "tag", items: refs.filter((r) => r.kind === "tag").sort((a, b) => b.name.localeCompare(a.name, undefined, { numeric: true })) });
  out.push({ key: "stash", label: t("sidebar.stashes"), icon: "box", items: refs.filter((r) => r.kind === "stash") });
  return out;
});

function shortName(g: Group, r: RefInfo) {
  return g.key.startsWith("remote:") ? r.name.slice(r.name.indexOf("/") + 1) : r.name;
}
</script>

<template>
  <aside class="side">
    <label class="filter">
      <Icon name="search" />
      <input id="ref-filter" v-model="filter" :placeholder="t('sidebar.filter')" spellcheck="false" />
    </label>
    <div class="groups">
      <section v-for="g in groups" :key="g.key" class="group">
        <button class="h" @click="collapsed[g.key] = !collapsed[g.key]">
          <Icon name="chev" :class="{ open: !collapsed[g.key] }" />
          <span>{{ g.label }}</span>
          <span class="n">{{ g.items.length }}</span>
        </button>
        <template v-if="!collapsed[g.key]">
          <button v-for="r in g.items" :key="r.full" class="item" :class="{ cur: r.head }" :title="r.name" @click="emit('goto', r.row)">
            <Icon :name="r.head ? 'check' : g.icon" />
            <span class="name">{{ shortName(g, r) }}</span>
            <span v-if="r.worktree" class="wtmark" :title="t('sidebar.wtElsewhere', { name: wtName(r.worktree), path: r.worktree })"><Icon name="wt" />{{ wtName(r.worktree) }}</span>
            <span v-if="r.synced_remote" class="sync" :title="t('sidebar.synced')">⇅</span>
          </button>
          <div v-if="!g.items.length" class="empty">{{ t("sidebar.none") }}</div>
        </template>
      </section>
      <section v-if="worktrees.length" class="group">
        <button class="h" @click="collapsed.wt = !collapsed.wt">
          <Icon name="chev" :class="{ open: !collapsed.wt }" />
          <span>{{ t("sidebar.worktrees") }}</span>
          <span class="n">{{ worktrees.length }}</span>
        </button>
        <template v-if="!collapsed.wt">
          <button
            v-for="w in worktrees"
            :key="w.path"
            class="wt"
            :class="{ cur: w.current, gone: w.prunable }"
            :title="w.prunable ? t('sidebar.wtGoneTitle', { path: w.path }) : w.current ? w.path : t('sidebar.wtOpen', { path: w.path })"
            @click="openWorktree(w)"
          >
            <Icon name="wt" />
            <span class="name">{{ w.name }} · {{ w.branch ?? t("sidebar.wtDetached") }}</span>
            <span v-if="w.current" class="tag">{{ t("sidebar.wtHere") }}</span>
            <span v-else-if="w.prunable" class="tag">{{ t("sidebar.wtGone") }}</span>
            <span v-else-if="w.locked" class="tag">{{ t("sidebar.wtLocked") }}</span>
            <span v-else-if="dirty?.[w.path]" class="tag mod">{{ t("sidebar.wtDirty") }}</span>
            <span class="p">{{ w.path }}</span>
          </button>
        </template>
      </section>
    </div>
  </aside>
</template>

<style scoped>
.side {
  background: var(--panel);
  border-right: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.filter {
  margin: 10px;
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
.groups {
  overflow-y: auto;
  padding-bottom: 12px;
}
.group {
  display: flex;
  flex-direction: column;
  margin-bottom: 6px;
}
.h {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border: 0;
  background: none;
  cursor: pointer;
  font-size: 10.5px;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--ink-3);
  font-weight: 600;
  text-align: left;
}
.h .i {
  width: 11px;
  height: 11px;
  transition: transform 0.12s;
}
.h .i.open {
  transform: rotate(90deg);
}
.h .n {
  margin-left: auto;
}
.item {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 4px 12px 4px 22px;
  border: 0;
  background: none;
  cursor: pointer;
  color: var(--ink-2);
  text-align: left;
  min-width: 0;
}
.item:hover {
  background: var(--hover);
  color: var(--ink);
}
.item.cur {
  color: var(--ink);
  font-weight: 600;
  background: var(--sel);
}
.item .name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item .sync {
  margin-left: auto;
  color: var(--ink-3);
  font-size: 11px;
}
.item .wtmark {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  flex: none;
  max-width: 45%;
  overflow: hidden;
  margin-left: auto;
  font-size: 11px;
  color: var(--ink-3);
}
.item .wtmark + .sync {
  margin-left: 6px;
}
.wt {
  display: grid;
  grid-template-columns: 16px minmax(0, 1fr) auto;
  align-items: center;
  gap: 1px 7px;
  padding: 5px 12px 5px 20px;
  border: 0;
  background: none;
  cursor: pointer;
  color: var(--ink-2);
  text-align: left;
}
.wt:hover {
  background: var(--hover);
  color: var(--ink);
}
.wt.cur {
  background: var(--sel);
  color: var(--ink);
  cursor: default;
}
.wt.cur > .i {
  color: var(--accent);
}
.wt.gone {
  opacity: 0.6;
  cursor: default;
}
.wt.gone .name {
  text-decoration: line-through;
}
.wt .name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}
.wt .p {
  grid-column: 2 / 4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  direction: rtl;
  text-align: left;
  font: 11px var(--f-mono);
  color: var(--ink-3);
}
.wt .tag {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 8px;
  color: var(--ink-3);
  background: color-mix(in srgb, var(--ink-3) 14%, transparent);
}
.wt .tag.mod {
  color: var(--warn);
  background: var(--warn-bg);
}
.empty {
  padding: 3px 22px;
  color: var(--ink-3);
  font-style: italic;
}
</style>
