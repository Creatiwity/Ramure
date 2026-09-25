<script setup lang="ts">
// Bandeau de mise à jour sous la barre d'outils (planche M) et toast « à jour ».
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import Icon from "./Icon.vue";
import type { Updater } from "../lib/updates";

const props = defineProps<{ updater: Updater; hasRepo: boolean }>();
const emit = defineEmits<{ install: [] }>();
const { t, locale } = useI18n();

const u = props.updater;
const notesOpen = ref(false);

const show = computed(() => {
  const p = u.phase.value;
  if (p === "downloading" || p === "installing" || p === "error") return true;
  return p === "available" && !!u.update.value && !u.dismissed.value;
});
const mb = (n: number) => (n / 1_048_576).toLocaleString(locale.value, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const size = computed(() => {
  const { done, total } = u.progress.value;
  return total ? t("updates.size", { done: mb(done), total: mb(total) }) : t("updates.sizeUnknown", { done: mb(done) });
});
const ratio = computed(() => {
  const { done, total } = u.progress.value;
  return total ? Math.min(1, done / total) : 0;
});
/** Échec pendant l'installation (une mise à jour était proposée) ou pendant la vérification. */
const failedTitle = computed(() => t(u.update.value ? "updates.failed" : "updates.checkFailed"));
const failedDetail = computed(() => {
  const d = (u.error.value ?? "").replace(/^Error:\s*/, "").trim();
  const sentence = d && !/[.!?]$/.test(d) ? `${d}.` : d;
  return u.update.value ? t("updates.failedDetail", { detail: sentence }) : sentence;
});
function retry() {
  if (u.update.value) emit("install");
  else u.check(true);
}

// Le toast « à jour » disparaît seul.
let timer: ReturnType<typeof setTimeout> | undefined;
watch(u.upToDate, (v) => {
  clearTimeout(timer);
  if (v) timer = setTimeout(() => (u.upToDate.value = null), 4000);
});
onBeforeUnmount(() => clearTimeout(timer));
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") notesOpen.value = false;
}
</script>

<template>
  <div v-if="show" class="upd" :class="{ ko: u.phase.value === 'error' }" :role="u.phase.value === 'error' ? 'alert' : 'status'">
    <template v-if="u.phase.value === 'available' && u.update.value">
      <span class="st"><Icon name="down" />{{ t("updates.available", { version: u.update.value.version }) }}</span>
      <span class="sub">
        {{ t("updates.current", { current: u.update.value.current }) }}
        <template v-if="u.update.value.notes"> · <button class="link" @click="notesOpen = true">{{ t("updates.notes") }}</button></template>
      </span>
      <span class="sp">
        <button class="btn quiet" @click="u.skip()">{{ t("updates.skip") }}</button>
        <button class="btn quiet" @click="u.later()">{{ t("updates.later") }}</button>
        <button class="btn primary" @click="emit('install')">{{ t("updates.install") }}</button>
      </span>
    </template>
    <template v-else-if="u.phase.value === 'downloading'">
      <span class="st"><Icon name="down" />{{ t("updates.downloading", { version: u.update.value?.version ?? "" }) }}</span>
      <span class="bar" role="progressbar" :aria-valuenow="Math.round(ratio * 100)" aria-valuemin="0" aria-valuemax="100"><i :style="{ width: `${ratio * 100}%` }"></i></span>
      <span class="sub">{{ size }}</span>
    </template>
    <template v-else-if="u.phase.value === 'installing'">
      <span class="st"><Icon name="refresh" />{{ t("updates.installing") }}</span>
      <span class="sub">{{ t(hasRepo ? "updates.installingRepo" : "updates.installingNoRepo") }}</span>
    </template>
    <template v-else>
      <span class="st"><Icon name="warn" />{{ failedTitle }}</span>
      <span class="sub err">{{ failedDetail }}</span>
      <span class="sp">
        <button class="btn quiet" @click="u.closeError()">{{ t("updates.close") }}</button>
        <button class="btn" @click="retry">{{ t("updates.retry") }}</button>
      </span>
    </template>
  </div>

  <div v-if="u.upToDate.value" class="uptodate" role="status"><Icon name="check" />{{ t("updates.upToDate", { version: u.upToDate.value }) }}</div>

  <div v-if="notesOpen && u.update.value" class="scrim" @click.self="notesOpen = false" @keydown="onKey">
    <div class="notes" role="dialog" aria-modal="true" :aria-label="t('updates.notesTitle', { version: u.update.value.version })">
      <header>
        <b>{{ t("updates.notesTitle", { version: u.update.value.version }) }}</b>
        <button class="btn quiet" :title="t('updates.close')" autofocus @click="notesOpen = false"><Icon name="x" /></button>
      </header>
      <pre>{{ u.update.value.notes }}</pre>
      <footer>
        <button class="btn quiet" @click="notesOpen = false">{{ t("updates.later") }}</button>
        <button class="btn primary" @click="(notesOpen = false), emit('install')">{{ t("updates.install") }}</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.upd {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 10px;
  padding: 7px 12px;
  background: color-mix(in srgb, var(--accent) 10%, var(--panel));
  border-bottom: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
}
.upd.ko {
  background: var(--warn-bg);
  border-bottom-color: color-mix(in srgb, var(--warn) 40%, transparent);
}
.st {
  display: flex;
  align-items: center;
  gap: 7px;
  font-weight: 600;
  color: var(--accent);
}
.ko .st {
  color: var(--warn);
}
.sub {
  color: var(--ink-2);
  min-width: 0;
}
.sub.err {
  user-select: text;
}
.sp {
  margin-left: auto;
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}
.link {
  border: 0;
  padding: 0;
  background: none;
  color: inherit;
  font: inherit;
  text-decoration: underline;
  cursor: pointer;
}
.bar {
  flex: 1 1 160px;
  max-width: 240px;
  height: 5px;
  border-radius: 3px;
  overflow: hidden;
  background: color-mix(in srgb, var(--accent) 22%, transparent);
}
.bar i {
  display: block;
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.2s;
}
.uptodate {
  position: fixed;
  left: 50%;
  bottom: 40px;
  transform: translateX(-50%);
  z-index: 40;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 14px;
  border-radius: 9px;
  background: var(--ink);
  color: var(--bg);
  box-shadow: var(--shadow);
}
.scrim {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: grid;
  place-items: center;
  padding: 16px;
  background: color-mix(in srgb, var(--ink) 30%, transparent);
}
.notes {
  width: min(620px, 100%);
  max-height: min(70vh, 640px);
  display: flex;
  flex-direction: column;
  background: var(--raise);
  border: 1px solid var(--line);
  border-radius: 10px;
  box-shadow: var(--shadow);
}
.notes header,
.notes footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
}
.notes header {
  justify-content: space-between;
  border-bottom: 1px solid var(--line);
}
.notes footer {
  justify-content: flex-end;
  border-top: 1px solid var(--line);
}
.notes pre {
  margin: 0;
  padding: 12px 14px;
  overflow: auto;
  white-space: pre-wrap;
  font: 12.5px/1.5 var(--f-ui, inherit);
  color: var(--ink);
  user-select: text;
}
</style>
