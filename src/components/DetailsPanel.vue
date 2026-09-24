<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { api, type Details, type FileChange } from "../api";
import { fullDate, initials } from "../lib/format";
import Icon from "./Icon.vue";

const props = defineProps<{ row: number | null; revision: number }>();
const emit = defineEmits<{ diff: [file: FileChange]; goto: [sha: string] }>();

const details = ref<Details | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const copied = ref(false);
let token = 0;

watch(
  () => [props.row, props.revision],
  async () => {
    const t = ++token;
    details.value = null;
    error.value = null;
    if (props.row == null) return;
    loading.value = true;
    try {
      const d = await api.commitDetails(props.row);
      if (t === token) details.value = d;
    } catch (e) {
      if (t === token) error.value = String(e);
    } finally {
      if (t === token) loading.value = false;
    }
  },
  { immediate: true },
);

const subject = computed(() => (details.value?.type === "commit" ? details.value.message.split("\n")[0] : ""));
const body = computed(() => (details.value?.type === "commit" ? details.value.message.split("\n").slice(1).join("\n").trim() : ""));
const totals = computed(() => {
  const files = details.value?.files ?? [];
  return { n: files.length, add: files.reduce((s, f) => s + (f.added ?? 0), 0), del: files.reduce((s, f) => s + (f.deleted ?? 0), 0) };
});
const signature = computed(() => {
  if (details.value?.type !== "commit") return null;
  const s = details.value.signature;
  if (s === "G") return { text: "vérifiée", ok: true };
  if (s === "N") return null;
  if (s === "U") return { text: "valide, confiance inconnue", ok: true };
  return { text: `non vérifiée (${s})`, ok: false };
});

function dir(path: string) {
  const i = path.lastIndexOf("/");
  return i >= 0 ? path.slice(0, i + 1) : "";
}
function base(path: string) {
  return path.slice(path.lastIndexOf("/") + 1);
}
async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1200);
  } catch {
    /* le presse-papiers peut être refusé : l'utilisateur sélectionne le sha à la main */
  }
}
</script>

<template>
  <aside class="details">
    <div v-if="row == null" class="empty">Sélectionnez un commit pour voir ses détails.</div>
    <div v-else-if="error" class="empty err">{{ error }}</div>
    <div v-else-if="!details" class="empty">{{ loading ? "Chargement…" : "" }}</div>
    <template v-else-if="details.type === 'commit'">
      <div class="sec">
        <div class="subj">{{ subject }}</div>
        <div v-if="body" class="body">{{ body }}</div>
      </div>
      <div class="sec">
        <dl class="kv">
          <dt>Auteur</dt>
          <dd><span class="av">{{ initials(details.author) }}</span>{{ details.author }} <span class="muted">{{ details.author_email }}</span></dd>
          <dt>Date</dt>
          <dd>{{ fullDate(details.author_time) }}</dd>
          <template v-if="details.committer !== details.author">
            <dt>Committer</dt>
            <dd>{{ details.committer }}</dd>
          </template>
          <dt>Commit</dt>
          <dd>
            <button class="sha" :title="details.id" @click="copy(details.id)">
              {{ details.id.slice(0, 10) }} <Icon name="copy" /><span v-if="copied" class="ok">copié</span>
            </button>
          </dd>
          <dt>{{ details.parents.length > 1 ? "Parents" : "Parent" }}</dt>
          <dd>
            <span v-if="!details.parents.length" class="muted">aucun (commit racine)</span>
            <button v-for="p in details.parents" :key="p" class="sha link" @click="emit('goto', p)">{{ p.slice(0, 10) }}</button>
          </dd>
          <template v-if="signature">
            <dt>Signature</dt>
            <dd :class="signature.ok ? 'good' : 'bad'">{{ signature.text }}</dd>
          </template>
        </dl>
      </div>
      <div class="sec files-head">
        <span class="eyebrow">{{ totals.n }} fichier{{ totals.n > 1 ? "s" : "" }}</span>
        <span class="num"><span class="a">+{{ totals.add }}</span> <span class="d">−{{ totals.del }}</span></span>
        <span v-if="details.parents.length > 1" class="muted">par rapport au premier parent</span>
      </div>
      <div class="files">
        <button v-for="f in details.files" :key="f.path" class="file" @click="emit('diff', f)">
          <span class="st" :class="f.status">{{ f.status }}</span>
          <span class="p" :title="f.old_path ? `${f.old_path} → ${f.path}` : f.path"><i>{{ dir(f.path) }}</i>{{ base(f.path) }}</span>
          <span class="num"><span v-if="f.added != null" class="a">+{{ f.added }}</span> <span v-if="f.deleted" class="d">−{{ f.deleted }}</span></span>
        </button>
      </div>
    </template>
    <template v-else>
      <div class="sec">
        <div class="subj">Changements non commités</div>
        <div class="body">
          Lecture seule : l'indexation et le commit se font dans votre IDE ou votre terminal.
          <span v-if="details.status.operation"><br />Opération en cours : <b>{{ details.status.operation }}</b>.</span>
        </div>
      </div>
      <div class="sec files-head">
        <span class="eyebrow">{{ details.files.length }} fichier{{ details.files.length > 1 ? "s" : "" }}</span>
      </div>
      <div class="files">
        <button v-for="f in details.files" :key="f.path" class="file" @click="emit('diff', f)">
          <span class="st" :class="f.status === '?' ? 'A' : f.status">{{ f.status === "?" ? "N" : f.status }}</span>
          <span class="p" :title="f.path"><i>{{ dir(f.path) }}</i>{{ base(f.path) }}</span>
          <span class="num muted">{{ f.status === "?" ? "non suivi" : "" }}</span>
        </button>
      </div>
    </template>
  </aside>
</template>

<style scoped>
.details {
  border-left: 1px solid var(--line);
  background: var(--panel);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
  user-select: text;
}
.empty {
  padding: 20px 16px;
  color: var(--ink-3);
}
.err {
  color: var(--del);
}
.sec {
  padding: 14px;
  border-bottom: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.subj {
  font-size: 13.5px;
  font-weight: 600;
  line-height: 1.35;
}
.body {
  color: var(--ink-2);
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}
.kv {
  display: grid;
  grid-template-columns: 72px 1fr;
  gap: 6px 8px;
  margin: 0;
  font-size: 12px;
}
.kv dt {
  color: var(--ink-3);
}
.kv dd {
  margin: 0;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.muted {
  color: var(--ink-3);
}
.good {
  color: var(--add);
}
.bad {
  color: var(--warn);
}
.av {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  display: inline-grid;
  place-items: center;
  font: 600 9px var(--f-ui);
  color: var(--accent-ink);
  background: var(--accent);
  vertical-align: -5px;
  margin-right: 6px;
}
.sha {
  font: 11.5px var(--f-mono);
  border: 0;
  background: none;
  padding: 0;
  cursor: pointer;
  color: var(--ink);
  display: inline-flex;
  gap: 5px;
  align-items: center;
  margin-right: 8px;
}
.sha .i {
  width: 12px;
  height: 12px;
  color: var(--ink-3);
}
.sha.link {
  color: var(--accent);
}
.ok {
  color: var(--add);
  font-family: var(--f-ui);
}
.files-head {
  flex-direction: row;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 4px 10px;
  white-space: nowrap;
  padding-bottom: 8px;
  border-bottom: 0;
}
.files {
  display: flex;
  flex-direction: column;
  padding-bottom: 12px;
}
.file {
  display: grid;
  grid-template-columns: 16px minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
  padding: 5px 14px;
  border: 0;
  background: none;
  text-align: left;
  cursor: pointer;
}
.file:hover {
  background: var(--hover);
}
.st {
  font: 600 10px var(--f-mono);
  text-align: center;
}
.st.M,
.st.T {
  color: var(--warn);
}
.st.A,
.st.C {
  color: var(--add);
}
.st.D,
.st.U {
  color: var(--del);
}
.st.R {
  color: var(--accent);
}
.p {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  direction: rtl;
  text-align: left;
}
.p i {
  font-style: normal;
  color: var(--ink-3);
}
.num {
  font: 11px var(--f-mono);
}
.a {
  color: var(--add);
}
.d {
  color: var(--del);
}
</style>
