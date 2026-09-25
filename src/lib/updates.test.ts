import { describe, expect, it } from "vitest";
import { createUpdater, DAY_S, type AvailableUpdate, type KeyValueStore, type UpdateBackend } from "./updates";

function memStore(init: Record<string, string> = {}): KeyValueStore & { data: Record<string, string> } {
  const data = { ...init };
  return {
    data,
    get: (k) => data[k] ?? null,
    set: (k, v) => {
      if (v == null) delete data[k];
      else data[k] = v;
    },
  };
}

function fakeUpdate(version = "0.3.0", fail?: string): AvailableUpdate {
  return {
    version,
    current: "0.2.0",
    notes: "Nouveautés",
    async install(onProgress) {
      await Promise.resolve(); // le téléchargement ne se termine jamais dans le même tick
      onProgress(0, 100);
      onProgress(40, 100);
      if (fail) throw new Error(fail);
      onProgress(100, 100);
    },
  };
}

function fakeBackend(opts: { supported?: boolean; update?: AvailableUpdate | null; fail?: boolean } = {}) {
  const calls = { check: 0, relaunch: 0 };
  const backend: UpdateBackend = {
    supported: async () => opts.supported ?? true,
    check: async () => {
      calls.check++;
      if (opts.fail) throw new Error("offline");
      return opts.update === undefined ? fakeUpdate() : opts.update;
    },
    currentVersion: async () => "0.2.0",
    relaunch: async () => {
      calls.relaunch++;
    },
  };
  return { backend, calls };
}

describe("mises à jour", () => {
  it("ne fait aucune requête si la build ne sait pas se mettre à jour", async () => {
    const { backend, calls } = fakeBackend({ supported: false });
    const u = createUpdater(backend, memStore());
    await u.init();
    await u.tick();
    await u.check(true);
    expect(calls.check).toBe(0);
  });

  it("vérifie au démarrage puis pas avant 24 h", async () => {
    let t = 1_000_000;
    const { backend, calls } = fakeBackend({ update: null });
    const store = memStore();
    const u = createUpdater(backend, store, () => t);
    await u.init();
    await u.tick();
    expect(calls.check).toBe(1);
    expect(store.data["ramure.updates.last"]).toBe(String(t));
    t += DAY_S - 1;
    await u.tick();
    expect(calls.check).toBe(1);
    t += 1;
    await u.tick();
    expect(calls.check).toBe(2);
  });

  it("aucune vérification automatique une fois désactivée, la manuelle reste possible", async () => {
    const { backend, calls } = fakeBackend({ update: null });
    const store = memStore();
    const u = createUpdater(backend, store);
    await u.init();
    u.setAuto(false);
    expect(store.data["ramure.updates.auto"]).toBe("0");
    await u.tick();
    expect(calls.check).toBe(0);
    await u.check(true);
    expect(calls.check).toBe(1);
    expect(u.upToDate.value).toBe("0.2.0");
    expect(createUpdater(backend, store).auto.value).toBe(false);
  });

  it("propose la mise à jour, « Ignorer cette version » tient jusqu'à la suivante", async () => {
    const store = memStore();
    const { backend } = fakeBackend();
    const u = createUpdater(backend, store);
    await u.init();
    await u.tick();
    expect(u.phase.value).toBe("available");
    expect(u.update.value?.version).toBe("0.3.0");
    u.skip();
    expect(store.data["ramure.updates.skip"]).toBe("0.3.0");

    const again = createUpdater(backend, store);
    await again.init();
    await again.tick();
    expect(again.update.value).toBeNull();
    // Demandée à la main, elle est proposée quand même.
    await again.check(true);
    expect(again.update.value?.version).toBe("0.3.0");

    const next = createUpdater(fakeBackend({ update: fakeUpdate("0.4.0") }).backend, memStore(store.data));
    await next.init();
    await next.check(false);
    expect(next.update.value?.version).toBe("0.4.0");
  });

  it("« Plus tard » masque le bandeau sans oublier la version", async () => {
    const u = createUpdater(fakeBackend().backend, memStore());
    await u.init();
    await u.check();
    u.later();
    expect(u.dismissed.value).toBe(true);
    expect(u.update.value?.version).toBe("0.3.0");
  });

  it("installe avec la progression puis relance", async () => {
    const { backend, calls } = fakeBackend();
    const u = createUpdater(backend, memStore());
    await u.init();
    await u.check();
    let saved = false;
    const seen: string[] = [];
    const p = u.install(() => (saved = true));
    seen.push(u.phase.value);
    await p;
    expect(seen[0]).toBe("downloading");
    expect(u.progress.value).toEqual({ done: 100, total: 100 });
    expect(u.phase.value).toBe("installing");
    expect(saved).toBe(true);
    expect(calls.relaunch).toBe(1);
  });

  it("un échec d'installation n'installe rien et permet de réessayer", async () => {
    const { backend, calls } = fakeBackend({ update: fakeUpdate("0.3.0", "signature invalide") });
    const u = createUpdater(backend, memStore());
    await u.init();
    await u.check();
    await u.install();
    expect(u.phase.value).toBe("error");
    expect(u.error.value).toContain("signature invalide");
    expect(calls.relaunch).toBe(0);
    u.closeError();
    expect(u.phase.value).toBe("available");
  });

  it("hors ligne : silencieux en automatique, signalé en manuel", async () => {
    const u = createUpdater(fakeBackend({ fail: true }).backend, memStore());
    await u.init();
    await u.tick();
    expect(u.phase.value).toBe("idle");
    expect(u.error.value).toBeNull();
    await u.check(true);
    expect(u.phase.value).toBe("error");
    expect(u.error.value).toContain("offline");
  });
});
