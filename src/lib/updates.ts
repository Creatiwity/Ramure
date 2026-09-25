// Mises à jour de Ramure (spec §4.3, planche M). Seule requête réseau de l'application : la
// lecture du manifeste de la dernière release, au démarrage puis toutes les 24 h, si la
// vérification automatique est active. Rien n'est téléchargé sans clic.
//
// La logique est isolée du plugin Tauri derrière `UpdateBackend`, pour être testée sans réseau.

import { ref, shallowRef } from "vue";

export interface AvailableUpdate {
  version: string;
  current: string;
  notes: string | null;
  /** Télécharge puis installe ; `onProgress` reçoit les octets reçus et le total s'il est connu. */
  install(onProgress: (done: number, total: number | null) => void): Promise<void>;
}

export interface UpdateBackend {
  /** Faux si cette build ne sait pas s'installer elle-même (paquet, `.deb`, développement). */
  supported(): Promise<boolean>;
  check(): Promise<AvailableUpdate | null>;
  currentVersion(): Promise<string>;
  relaunch(): Promise<void>;
}

export interface KeyValueStore {
  get(k: string): string | null;
  set(k: string, v: string | null): void;
}

export type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "error";

export const DAY_S = 24 * 3600;
const K_AUTO = "ramure.updates.auto";
const K_SKIP = "ramure.updates.skip";
const K_LAST = "ramure.updates.last";

export function createUpdater(backend: UpdateBackend, store: KeyValueStore, now: () => number = () => Date.now() / 1000) {
  const supported = ref(false);
  const auto = ref(store.get(K_AUTO) !== "0");
  const phase = ref<UpdatePhase>("idle");
  const update = shallowRef<AvailableUpdate | null>(null);
  const progress = ref<{ done: number; total: number | null }>({ done: 0, total: null });
  const error = ref<string | null>(null);
  /** Version courante, après une vérification manuelle sans nouveauté (toast « à jour »). */
  const upToDate = ref<string | null>(null);
  const lastCheck = ref<number | null>(Number(store.get(K_LAST)) || null);
  /** « Plus tard » : bandeau masqué jusqu'au prochain démarrage. */
  const dismissed = ref(false);

  async function init() {
    supported.value = await backend.supported().catch(() => false);
  }

  /** Vérifie maintenant. `manual` : demandée par l'utilisateur, ignore « Ignorer cette version ». */
  async function check(manual = false) {
    if (!supported.value || phase.value === "checking" || phase.value === "downloading" || phase.value === "installing") return;
    phase.value = "checking";
    error.value = null;
    upToDate.value = null;
    try {
      const u = await backend.check();
      lastCheck.value = Math.floor(now());
      store.set(K_LAST, String(lastCheck.value));
      if (u && (manual || store.get(K_SKIP) !== u.version)) {
        update.value = u;
        dismissed.value = false;
        phase.value = "available";
      } else {
        update.value = null;
        phase.value = "idle";
        if (manual) upToDate.value = u?.current ?? (await backend.currentVersion().catch(() => "?"));
      }
    } catch (e) {
      // Une vérification automatique qui échoue (hors ligne) reste silencieuse.
      phase.value = manual ? "error" : "idle";
      if (manual) error.value = String(e);
    }
  }

  /** Vérification automatique : seulement si activée et si la dernière date de plus de 24 h. */
  async function tick() {
    if (!auto.value || phase.value !== "idle" || update.value) return;
    if (lastCheck.value != null && now() - lastCheck.value < DAY_S) return;
    await check(false);
  }

  /** Télécharge, installe et relance. `beforeRelaunch` mémorise ce qu'il faut rouvrir. */
  async function install(beforeRelaunch?: () => void) {
    const u = update.value;
    if (!u) return;
    phase.value = "downloading";
    error.value = null;
    progress.value = { done: 0, total: null };
    try {
      await u.install((done, total) => {
        progress.value = { done, total };
        if (total != null && done >= total) phase.value = "installing";
      });
      phase.value = "installing";
      beforeRelaunch?.();
      await backend.relaunch();
    } catch (e) {
      phase.value = "error";
      error.value = String(e);
    }
  }

  function later() {
    dismissed.value = true;
  }

  function skip() {
    if (update.value) store.set(K_SKIP, update.value.version);
    update.value = null;
    phase.value = "idle";
  }

  function closeError() {
    error.value = null;
    phase.value = update.value ? "available" : "idle";
    if (update.value) dismissed.value = true;
  }

  function setAuto(on: boolean) {
    auto.value = on;
    store.set(K_AUTO, on ? "1" : "0");
  }

  return { supported, auto, phase, update, progress, error, upToDate, lastCheck, dismissed, init, check, tick, install, later, skip, closeError, setAuto };
}

export type Updater = ReturnType<typeof createUpdater>;

/** Backend réel : plugins updater et process de Tauri, chargés à la demande. */
export const tauriBackend: UpdateBackend = {
  async supported() {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<boolean>("updates_supported");
  },
  async check() {
    const { check } = await import("@tauri-apps/plugin-updater");
    const u = await check();
    if (!u) return null;
    return {
      version: u.version,
      current: u.currentVersion,
      notes: u.body?.trim() || null,
      async install(onProgress) {
        let done = 0;
        let total: number | null = null;
        await u.downloadAndInstall((ev) => {
          if (ev.event === "Started") total = ev.data.contentLength ?? null;
          else if (ev.event === "Progress") done += ev.data.chunkLength;
          else if (ev.event === "Finished" && total == null) total = done;
          onProgress(done, total);
        });
      },
    };
  },
  async currentVersion() {
    const { getVersion } = await import("@tauri-apps/api/app");
    return getVersion();
  },
  async relaunch() {
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  },
};

/** Hors Tauri (navigateur, tests) : aucune mise à jour possible. */
export const noBackend: UpdateBackend = {
  supported: async () => false,
  check: async () => null,
  currentVersion: async () => "dev",
  relaunch: async () => {},
};
