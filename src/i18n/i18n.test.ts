import { describe, expect, it } from "vitest";
import fr from "./fr";
import en from "./en";
import { errorText, i18n } from "./index";

function keys(o: object, prefix = ""): string[] {
  return Object.entries(o).flatMap(([k, v]) => (typeof v === "object" ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`]));
}
function placeholders(s: string): string[] {
  return [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}

describe("traductions", () => {
  it("fr et en ont exactement les mêmes clés", () => {
    expect(keys(en).sort()).toEqual(keys(fr).sort());
  });
  it("chaque traduction garde les mêmes variables et le même nombre de formes de pluriel", () => {
    const get = (o: object, k: string) => k.split(".").reduce((x: any, p) => x[p], o) as string;
    for (const k of keys(fr)) {
      expect(placeholders(get(en, k)), k).toEqual(placeholders(get(fr, k)));
      expect(get(en, k).split("|").length, k).toBe(get(fr, k).split("|").length);
    }
  });
  it("aucune chaîne vide", () => {
    const get = (o: object, k: string) => k.split(".").reduce((x: any, p) => x[p], o) as string;
    for (const k of keys(fr)) {
      expect(get(fr, k).trim(), k).not.toBe("");
      expect(get(en, k).trim(), k).not.toBe("");
    }
  });
});

describe("erreurs", () => {
  it("traduit les codes du cœur et garde le détail", () => {
    i18n.global.locale.value = "fr";
    expect(errorText({ code: "not_found", detail: "/tmp/x" })).toBe("Dossier introuvable : /tmp/x.");
    i18n.global.locale.value = "en";
    expect(errorText({ code: "not_found", detail: "/tmp/x" })).toBe("Folder not found: /tmp/x.");
    expect(errorText({ code: "inconnu", detail: "boom" })).toBe("Error: boom");
    expect(errorText(new Error("oops"))).toBe("Error: oops");
  });
  it("pluriels", () => {
    i18n.global.locale.value = "fr";
    expect(i18n.global.t("details.files", 1)).toBe("1 fichier");
    expect(i18n.global.t("details.files", 3)).toBe("3 fichiers");
    i18n.global.locale.value = "en";
    expect(i18n.global.t("graph.wipChanged", 2)).toBe("2 changed files");
  });
});
