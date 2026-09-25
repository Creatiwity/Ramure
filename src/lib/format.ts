// Mise en forme des lignes du graph (ux-graph.md, D5 et D6).

export interface ParsedMessage {
  /** Type Conventional Commits (`feat`, `fix`…), ou `merge`. */
  type: string | null;
  scope: string | null;
  subject: string;
  /** Branche mergée, pour les merges réécrits en forme courte. */
  merged: string | null;
  pr: string | null;
  /** Décalage (en caractères) du sujet dans le message d'origine, pour le surlignage. */
  offset: number;
  /** Décalage de la portée (entre parenthèses) dans le message d'origine. */
  scopeOffset: number;
}

const CC = /^(\w+)(?:\(([^)]+)\))?!?:\s*/;

export function parseMessage(summary: string): ParsedMessage {
  const pr = summary.match(/^Merge pull request #(\d+) from (\S+)/);
  if (pr) {
    const from = pr[2];
    const slash = from.indexOf("/");
    return { type: "merge", scope: null, subject: summary, merged: slash >= 0 ? from.slice(slash + 1) : from, pr: pr[1], offset: 0, scopeOffset: 0 };
  }
  const mb = summary.match(/^Merge (?:remote-tracking )?branch '([^']+)'/);
  if (mb) return { type: "merge", scope: null, subject: summary, merged: mb[1], pr: null, offset: 0, scopeOffset: 0 };
  const cc = summary.match(CC);
  if (cc) {
    return {
      type: cc[1].toLowerCase(),
      scope: cc[2] ?? null,
      subject: summary.slice(cc[0].length),
      merged: null,
      pr: null,
      offset: [...cc[0]].length,
      scopeOffset: [...cc[1]].length + 1,
    };
  }
  return { type: null, scope: null, subject: summary, merged: null, pr: null, offset: 0, scopeOffset: 0 };
}

/** Le dépôt suit-il Conventional Commits ? (plus de 60 % des messages échantillonnés) */
export function usesConventionalCommits(summaries: string[]): boolean {
  const relevant = summaries.filter((s) => !s.startsWith("Merge "));
  if (relevant.length < 5) return false;
  const ok = relevant.filter((s) => CC.test(s)).length;
  return ok / relevant.length > 0.6;
}

/** Découpe un texte en segments surlignés ou non à partir de positions en caractères. */
export function splitHighlights(text: string, indices: number[], offset = 0): { text: string; hit: boolean }[] {
  const chars = [...text];
  const set = new Set(indices.map((i) => i - offset));
  const out: { text: string; hit: boolean }[] = [];
  chars.forEach((c, i) => {
    const hit = set.has(i);
    const last = out[out.length - 1];
    if (last && last.hit === hit) last.text += c;
    else out.push({ text: c, hit });
  });
  return out;
}

const DAY = 86400;
const WEEKDAYS = ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."];
const MONTHS = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];

/** Clé du jour, pour n'afficher la date qu'à son changement. */
export function dayKey(time: number): string {
  const d = new Date(time * 1000);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

/** « 17:02 » aujourd'hui, « hier », « lun. 21 » cette semaine, « 21 sept. » cette année, sinon « 21/09/2024 ». */
export function relativeDate(time: number, now = Date.now() / 1000): string {
  const d = new Date(time * 1000);
  const n = new Date(now * 1000);
  const startOfToday = new Date(n.getFullYear(), n.getMonth(), n.getDate()).getTime() / 1000;
  if (time >= startOfToday) return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  if (time >= startOfToday - DAY) return "hier";
  if (time >= startOfToday - 6 * DAY) return `${WEEKDAYS[d.getDay()]} ${d.getDate()}`;
  if (d.getFullYear() === n.getFullYear()) return `${d.getDate()} ${MONTHS[d.getMonth()]}`;
  return `${String(d.getDate()).padStart(2, "0")}/${String(d.getMonth() + 1).padStart(2, "0")}/${d.getFullYear()}`;
}

export function fullDate(time: number): string {
  return new Date(time * 1000).toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });
}

/** Tronque au milieu : `feature/…/scoring-v2`. */
export function middleEllipsis(s: string, max: number): string {
  if ([...s].length <= max) return s;
  const chars = [...s];
  const keep = max - 1;
  const head = Math.ceil(keep * 0.45);
  const tail = keep - head;
  return chars.slice(0, head).join("") + "…" + chars.slice(chars.length - tail).join("");
}

export function initials(name: string): string {
  const parts = name.replace(/[^\p{L}\s.-]/gu, "").split(/[\s.-]+/).filter(Boolean);
  return ((parts[0]?.[0] ?? "?") + (parts[1]?.[0] ?? "")).toUpperCase();
}

/** « à l'instant », « il y a 5 min », « il y a 2 h », « hier », « il y a 3 j », puis la date. */
export function ago(time: number, now = Date.now() / 1000): string {
  const d = Math.max(0, now - time);
  if (d < 60) return "à l'instant";
  if (d < 3600) return `il y a ${Math.floor(d / 60)} min`;
  if (d < 86400) return `il y a ${Math.floor(d / 3600)} h`;
  if (d < 2 * 86400) return "hier";
  if (d < 30 * 86400) return `il y a ${Math.floor(d / 86400)} j`;
  return relativeDate(time, now);
}

/** Dernier segment d'un chemin. */
export function basename(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || path;
}

/** Chemin relatif à une racine (« clients/acme/api »), ou le chemin avec ~ pour le dossier personnel. */
export function relativeTo(path: string, root: string | null, home?: string): string {
  if (root && (path === root || path.startsWith(root.replace(/[\\/]+$/, "") + "/"))) return path.slice(root.replace(/[\\/]+$/, "").length + 1) || basename(path);
  if (home && path.startsWith(home + "/")) return "~" + path.slice(home.length);
  return path;
}
