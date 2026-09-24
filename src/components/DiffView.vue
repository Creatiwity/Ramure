<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { api, type FileChange } from "../api";
import Icon from "./Icon.vue";

const props = defineProps<{ row: number; file: FileChange; wip: boolean }>();
const emit = defineEmits<{ close: [] }>();

const raw = ref("");
const error = ref<string | null>(null);

watch(
  () => [props.row, props.file.path],
  async () => {
    raw.value = "";
    error.value = null;
    try {
      raw.value = await api.fileDiff(props.row, props.file.path, props.file.old_path, props.file.status === "?");
    } catch (e) {
      error.value = String(e);
    }
  },
  { immediate: true },
);

interface Line {
  kind: "add" | "del" | "ctx" | "hunk" | "meta";
  old: number | null;
  new: number | null;
  text: string;
}

/** Diff unifié → lignes numérotées. L'en-tête git (diff --git, index, ---/+++) est masqué. */
const lines = computed<Line[]>(() => {
  const out: Line[] = [];
  let o = 0;
  let n = 0;
  let inHunk = false;
  for (const l of raw.value.split("\n")) {
    const m = l.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/);
    if (m) {
      o = Number(m[1]);
      n = Number(m[2]);
      inHunk = true;
      out.push({ kind: "hunk", old: null, new: null, text: m[3].trim() || "…" });
    } else if (!inHunk) {
      if (l.startsWith("Binary files")) out.push({ kind: "meta", old: null, new: null, text: "Fichier binaire" });
    } else if (l.startsWith("+")) out.push({ kind: "add", old: null, new: n++, text: l.slice(1) });
    else if (l.startsWith("-")) out.push({ kind: "del", old: o++, new: null, text: l.slice(1) });
    else if (l.startsWith(" ")) out.push({ kind: "ctx", old: o++, new: n++, text: l.slice(1) });
    else if (l.startsWith("\\")) out.push({ kind: "meta", old: null, new: null, text: "Pas de retour à la ligne en fin de fichier" });
  }
  return out;
});

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <section class="diff">
    <header>
      <Icon name="file" />
      <b class="path">{{ file.path }}</b>
      <span v-if="file.old_path" class="muted">renommé depuis {{ file.old_path }}</span>
      <span v-if="wip" class="muted">· non commité</span>
      <button class="btn quiet close" title="Fermer (Échap)" @click="emit('close')"><Icon name="x" /> Fermer <kbd>Échap</kbd></button>
    </header>
    <div v-if="error" class="msg err">{{ error }}</div>
    <div v-else-if="!raw" class="msg">Chargement…</div>
    <div v-else-if="!lines.length" class="msg">Aucune différence textuelle.</div>
    <div v-else class="code">
      <div v-for="(l, i) in lines" :key="i" class="ln" :class="l.kind">
        <span class="no">{{ l.old ?? "" }}</span><span class="no">{{ l.new ?? "" }}</span
        ><span class="sign">{{ l.kind === "add" ? "+" : l.kind === "del" ? "−" : "" }}</span><span class="t">{{ l.text }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.diff {
  position: absolute;
  inset: 0;
  background: var(--bg);
  display: flex;
  flex-direction: column;
  z-index: 10;
  min-height: 0;
}
header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px 0 14px;
  height: 38px;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
  flex: none;
}
.path {
  font-family: var(--f-mono);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.muted {
  color: var(--ink-3);
  white-space: nowrap;
}
.close {
  margin-left: auto;
}
.msg {
  padding: 20px;
  color: var(--ink-3);
}
.err {
  color: var(--del);
}
.code {
  overflow: auto;
  flex: 1;
  font: 12px/1.55 var(--f-mono);
  user-select: text;
  padding-bottom: 20px;
}
.ln {
  display: grid;
  grid-template-columns: 48px 48px 18px 1fr;
  min-width: max-content;
}
.no {
  color: var(--ink-3);
  text-align: right;
  padding-right: 8px;
  user-select: none;
  opacity: 0.8;
}
.sign {
  text-align: center;
  user-select: none;
}
.t {
  white-space: pre;
  padding-right: 24px;
}
.add {
  background: color-mix(in srgb, var(--add) 13%, transparent);
}
.add .sign {
  color: var(--add);
}
.del {
  background: color-mix(in srgb, var(--del) 12%, transparent);
}
.del .sign {
  color: var(--del);
}
.hunk {
  background: var(--panel);
  color: var(--ink-3);
  margin-top: 6px;
}
.hunk .t {
  font-style: italic;
}
.meta {
  color: var(--ink-3);
  font-style: italic;
}
</style>
