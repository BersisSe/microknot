import type { Knot, Connection } from "./types";

const COL_WIDTH = 220;
const ROW_HEIGHT = 100;

// Auto-layout: left-to-right by dependency depth (topological-ish pass).
// Position is never persisted server-side, so this runs fresh every load.
export function layoutKnots(
  knots: Knot[],
  connections: Connection[],
): { id: string; x: number; y: number }[] {
  const incoming: Record<string, number> = {};
  knots.forEach((k) => (incoming[k.id] = 0));
  connections.forEach((c) => {
    if (incoming[c.to] !== undefined) incoming[c.to] += 1;
  });

  const colOf: Record<string, number> = {};
  let frontier = knots.filter((k) => incoming[k.id] === 0).map((k) => k.id);
  frontier.forEach((id) => (colOf[id] = 0));

  let guard = 0;
  while (frontier.length && guard < 200) {
    guard += 1;
    const next: string[] = [];
    frontier.forEach((id) => {
      connections.forEach((c) => {
        if (c.from === id) {
          const candidate = (colOf[id] ?? 0) + 1;
          if (colOf[c.to] === undefined || colOf[c.to] < candidate)
            colOf[c.to] = candidate;
          if (!next.includes(c.to)) next.push(c.to);
        }
      });
    });
    frontier = next;
  }
  knots.forEach((k) => {
    if (colOf[k.id] === undefined) colOf[k.id] = 0;
  });

  const colCounts: Record<number, number> = {};
  return knots.map((k) => {
    const col = colOf[k.id];
    const row = colCounts[col] || 0;
    colCounts[col] = row + 1;
    return { id: k.id, x: 40 + col * COL_WIDTH, y: 40 + row * ROW_HEIGHT };
  });
}
