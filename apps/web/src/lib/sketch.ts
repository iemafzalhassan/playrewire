// Small helpers for drawing "by hand" in SVG: seeded jitter, smooth curves
// through points, and closed organic blobs. Deterministic, so a district looks
// the same on every visit.

export type Pt = [number, number];

/** Mulberry32: tiny seeded PRNG in [0, 1). */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const f = (n: number) => Math.round(n * 10) / 10;

/** Catmull-Rom spline through `pts` as cubic Bezier path data. */
export function smoothPath(pts: Pt[], closed = false): string {
  if (pts.length < 2) return "";
  const p = (i: number): Pt =>
    closed ? pts[(i + pts.length) % pts.length] : pts[Math.max(0, Math.min(pts.length - 1, i))];
  const n = closed ? pts.length : pts.length - 1;
  let d = `M${f(pts[0][0])} ${f(pts[0][1])}`;
  for (let i = 0; i < n; i++) {
    const [p0, p1, p2, p3] = [p(i - 1), p(i), p(i + 1), p(i + 2)];
    const c1: Pt = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2: Pt = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    d += ` C${f(c1[0])} ${f(c1[1])} ${f(c2[0])} ${f(c2[1])} ${f(p2[0])} ${f(p2[1])}`;
  }
  return closed ? d + " Z" : d;
}

/** A closed, slightly lumpy shape around (cx, cy). */
export function blob(cx: number, cy: number, rx: number, ry: number, seed: number, points = 9): string {
  const r = rng(seed);
  const pts: Pt[] = [];
  const turn = r() * Math.PI;
  for (let i = 0; i < points; i++) {
    const a = turn + (i / points) * Math.PI * 2;
    const k = 0.84 + r() * 0.26;
    pts.push([cx + Math.cos(a) * rx * k, cy + Math.sin(a) * ry * k]);
  }
  return smoothPath(pts, true);
}
