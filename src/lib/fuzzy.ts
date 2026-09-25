// Correspondance floue pour la palette de commandes (listes courtes : dépôts, branches, actions).
// Les caractères de la requête doivent apparaître dans l'ordre ; le score favorise les débuts de
// mot, les suites consécutives et les textes courts.

export interface FuzzyMatch {
  score: number;
  indices: number[];
}

const SEPARATORS = new Set([" ", "/", "-", "_", ".", ":", "(", ")"]);

function fold(s: string): string {
  return s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

function greedy(q: string[], t: string[], start: number): number[] | null {
  const indices: number[] = [];
  let ti = start;
  for (const ch of q) {
    let found = -1;
    for (let k = ti; k < t.length; k++) {
      if (t[k] !== ch) continue;
      if (found < 0) found = k;
      // Caractère suivant immédiat ou début de mot : on le prend tout de suite.
      if (k === ti || k === 0 || SEPARATORS.has(t[k - 1])) {
        found = k;
        break;
      }
    }
    if (found < 0) return null;
    indices.push(found);
    ti = found + 1;
  }
  return indices;
}

function scoreOf(indices: number[], t: string[]): number {
  let score = 0;
  indices.forEach((k, i) => {
    score += 1;
    if (k === 0 || SEPARATORS.has(t[k - 1])) score += 8;
    if (i > 0 && indices[i - 1] === k - 1) score += 5;
  });
  return score - indices[0] * 0.5 - t.length * 0.05;
}

export function fuzzy(query: string, text: string): FuzzyMatch | null {
  const q = [...fold(query.replace(/\s+/g, ""))];
  if (!q.length) return { score: 0, indices: [] };
  const t = [...fold(text)];
  // Chaque occurrence du premier caractère est un départ possible ; on garde le meilleur.
  let best: FuzzyMatch | null = null;
  for (let start = 0; start < t.length; start++) {
    if (t[start] !== q[0]) continue;
    const indices = greedy(q, t, start);
    if (!indices) break; // si ça échoue depuis ici, ça échouera plus loin aussi
    const score = scoreOf(indices, t);
    if (!best || score > best.score) best = { score, indices };
  }
  return best;
}

/** Filtre et trie des éléments ; à score égal, l'ordre d'origine est conservé. */
export function rank<T>(items: T[], query: string, text: (item: T) => string): { item: T; match: FuzzyMatch }[] {
  const out: { item: T; match: FuzzyMatch; i: number }[] = [];
  items.forEach((item, i) => {
    const m = fuzzy(query, text(item));
    if (m) out.push({ item, match: m, i });
  });
  if (query.trim()) out.sort((a, b) => b.match.score - a.match.score || a.i - b.i);
  return out;
}
