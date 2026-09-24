// Couleurs de lanes lues depuis les jetons CSS, pour le canvas du graph.
// Les versions désaturées (mode focus, ux-graph.md D4) sont calculées par mélange avec la
// couleur des filets, comme `color-mix(in srgb, var(--lN) 34%, var(--line))` dans la planche.

export interface Palette {
  vivid: string[];
  muted: string[];
  neutral: string;
  bg: string;
  line: string;
  mark: string;
}

function hex(v: string): [number, number, number] {
  const s = v.trim().replace("#", "");
  const full = s.length === 3 ? s.split("").map((c) => c + c).join("") : s;
  const n = parseInt(full.slice(0, 6), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function mix(a: string, b: string, ratio: number): string {
  const [r1, g1, b1] = hex(a);
  const [r2, g2, b2] = hex(b);
  const m = (x: number, y: number) => Math.round(x * ratio + y * (1 - ratio));
  return `#${[m(r1, r2), m(g1, g2), m(b1, b2)].map((x) => x.toString(16).padStart(2, "0")).join("")}`;
}

export function readPalette(el: Element = document.documentElement): Palette {
  const cs = getComputedStyle(el);
  const get = (n: string) => cs.getPropertyValue(n).trim() || "#888888";
  const line = get("--line");
  const vivid = Array.from({ length: 8 }, (_, i) => get(`--l${i}`));
  return {
    vivid,
    muted: vivid.map((c) => mix(c, line, 0.34)),
    neutral: get("--lx"),
    bg: get("--bg"),
    line,
    mark: get("--mark"),
  };
}
