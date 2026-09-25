import { describe, expect, it } from "vitest";
import { middleEllipsis, parseMessage, relativeDate, splitHighlights, usesConventionalCommits, initials } from "./format";
import { fuzzyIncludes, highlightIndices } from "../api";

/** Intl utilise des espaces insécables : on les compare comme des espaces. */
const sp = (s: string) => s.replace(/[\u00a0\u202f]/g, " ");

describe("parseMessage", () => {
  it("sépare type, portée et sujet", () => {
    const m = parseMessage("feat(scoring): pondère les tâches en retard");
    expect(m).toMatchObject({ type: "feat", scope: "scoring", subject: "pondère les tâches en retard", offset: 15, scopeOffset: 5 });
  });
  it("réécrit les merges de PR", () => {
    expect(parseMessage("Merge pull request #142 from creatiwity/fix/pwa-cache")).toMatchObject({ type: "merge", merged: "fix/pwa-cache", pr: "142" });
    expect(parseMessage("Merge branch 'feature/onboarding' into main")).toMatchObject({ type: "merge", merged: "feature/onboarding" });
  });
  it("laisse les messages libres intacts", () => {
    expect(parseMessage("Initial commit")).toMatchObject({ type: null, subject: "Initial commit" });
  });
});

describe("usesConventionalCommits", () => {
  it("détecte un dépôt conventionnel en ignorant les merges", () => {
    expect(usesConventionalCommits(["feat: a", "fix(x): b", "Merge branch 'y'", "chore: c", "docs: d", "test: e", "oups"])).toBe(true);
    expect(usesConventionalCommits(["a", "b", "c", "d", "feat: e"])).toBe(false);
  });
});

describe("splitHighlights", () => {
  it("regroupe les caractères consécutifs, avec décalage", () => {
    expect(splitHighlights("abcd", [16, 17], 15)).toEqual([
      { text: "a", hit: false },
      { text: "bc", hit: true },
      { text: "d", hit: false },
    ]);
  });
});

describe("relativeDate", () => {
  const now = new Date(2026, 8, 24, 18, 0).getTime() / 1000;
  const at = (...a: [number, number, number, number, number]) => new Date(...a).getTime() / 1000;
  it("affiche l'heure aujourd'hui, « hier », le jour de la semaine, puis la date (fr)", () => {
    expect(sp(relativeDate(at(2026, 8, 24, 17, 2), "fr", now))).toBe("17:02");
    expect(sp(relativeDate(at(2026, 8, 23, 9, 0), "fr", now))).toBe("hier");
    expect(sp(relativeDate(at(2026, 8, 21, 9, 0), "fr", now))).toBe("lun. 21");
    expect(sp(relativeDate(at(2026, 5, 2, 9, 0), "fr", now))).toBe("2 juin");
    expect(sp(relativeDate(at(2024, 8, 21, 9, 0), "fr", now))).toBe("21/09/2024");
  });
  it("mêmes règles en anglais", () => {
    expect(sp(relativeDate(at(2026, 8, 24, 17, 2), "en", now))).toBe("5:02 PM");
    expect(sp(relativeDate(at(2026, 8, 23, 9, 0), "en", now))).toBe("yesterday");
    expect(sp(relativeDate(at(2026, 8, 21, 9, 0), "en", now))).toBe("Mon 21");
    expect(sp(relativeDate(at(2026, 5, 2, 9, 0), "en", now))).toBe("Jun 2");
    expect(sp(relativeDate(at(2024, 8, 21, 9, 0), "en", now))).toBe("09/21/2024");
  });
});

describe("divers", () => {
  it("tronque au milieu", () => {
    expect(middleEllipsis("feature/scoring-urgency-v2", 16)).toBe("feature…gency-v2");
    expect(middleEllipsis("main", 16)).toBe("main");
  });
  it("initiales", () => {
    expect(initials("Julien B.")).toBe("JB");
    expect(initials("léa")).toBe("L");
  });
  it("recherche floue de démonstration", () => {
    expect(fuzzyIncludes("pondère les tâches en retard", "pndr")).toBe(true);
    expect(highlightIndices("pwa cache", ["pwa"])).toEqual([0, 1, 2]);
  });
});
