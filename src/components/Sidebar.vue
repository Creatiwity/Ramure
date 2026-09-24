<script setup lang="ts">
import { computed, ref } from "vue";
import type { RefInfo, RepoSummary } from "../api";
import Icon from "./Icon.vue";

const props = defineProps<{ summary: RepoSummary }>();
const emit = defineEmits<{ goto: [row: number] }>();

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
  const out: Group[] = [{ key: "local", label: "Local", icon: "laptop", items: refs.filter((r) => r.kind === "local").sort(byName) }];
  const remotes = new Map<string, RefInfo[]>();
  for (const r of refs.filter((r) => r.kind === "remote")) {
    const name = r.name.split("/")[0];
    remotes.set(name, [...(remotes.get(name) ?? []), r]);
  }
  for (const [name, items] of [...remotes].sort()) out.push({ key: `remote:${name}`, label: name, icon: "cloud", items: items.sort(byName) });
  out.push({ key: "tags", label: "Tags", icon: "tag", items: refs.filter((r) => r.kind === "tag").sort((a, b) => b.name.localeCompare(a.name, undefined, { numeric: true })) });
  out.push({ key: "stash", label: "Stashes", icon: "box", items: refs.filter((r) => r.kind === "stash") });
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
      <input id="ref-filter" v-model="filter" placeholder="Filtrer les refs" spellcheck="false" />
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
            <span v-if="r.synced_remote" class="sync" title="Identique à sa branche distante">⇅</span>
          </button>
          <div v-if="!g.items.length" class="empty">Aucune</div>
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
.empty {
  padding: 3px 22px;
  color: var(--ink-3);
  font-style: italic;
}
</style>
