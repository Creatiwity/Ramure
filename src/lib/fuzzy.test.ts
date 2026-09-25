import { describe, expect, it } from "vitest";
import { fuzzy, rank } from "./fuzzy";
import { ago, basename, relativeTo } from "./format";

describe("fuzzy", () => {
  it("trouve une sous-séquence insensible à la casse et aux accents", () => {
    expect(fuzzy("rmr", "Ramure")?.indices).toEqual([0, 2, 4]);
    expect(fuzzy("equipe", "Équipe")).not.toBeNull();
    expect(fuzzy("xyz", "ramure")).toBeNull();
  });
  it("préfère les débuts de mot", () => {
    expect(fuzzy("fs", "feature/scoring")?.indices).toEqual([0, 8]);
  });
  it("classe les débuts de mot et les suites consécutives devant", () => {
    const items = ["incubator", "clients/acme/api", "api-gateway", "rapidocs"];
    expect(rank(items, "api", (x) => x).map((r) => r.item)).toEqual(["api-gateway", "clients/acme/api", "rapidocs"]);
  });
  it("garde l'ordre d'origine sans requête", () => {
    expect(rank(["b", "a"], "", (x) => x).map((r) => r.item)).toEqual(["b", "a"]);
  });
});

describe("ago / chemins", () => {
  const now = 1_790_000_000;
  it("formate une durée écoulée", () => {
    expect(ago(now - 10, now)).toBe("à l'instant");
    expect(ago(now - 5 * 60, now)).toBe("il y a 5 min");
    expect(ago(now - 2 * 3600, now)).toBe("il y a 2 h");
    expect(ago(now - 30 * 3600, now)).toBe("hier");
    expect(ago(now - 3 * 86400, now)).toBe("il y a 3 j");
  });
  it("affiche un chemin relatif à la racine", () => {
    expect(relativeTo("/home/j/code/clients/acme/api", "/home/j/code")).toBe("clients/acme/api");
    expect(relativeTo("/home/j/perso/x", "/home/j/code", "/home/j")).toBe("~/perso/x");
    expect(basename("/home/j/code/ramure/")).toBe("ramure");
  });
});
