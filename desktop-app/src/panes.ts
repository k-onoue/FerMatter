import { uid } from "./api";
export type Leaf = { id: string; tabs: string[]; active: string };
export type Split = {
  id: string;
  direction: "row" | "column";
  children: [Pane, Pane];
  ratio: number;
};
export type Pane = Leaf | Split;
export const leaf = (path = ""): Leaf => ({
  id: uid(),
  tabs: path ? [path] : [],
  active: path,
});
export function leaves(p: Pane): Leaf[] {
  return "children" in p ? p.children.flatMap(leaves) : [p];
}
export function update(p: Pane, id: string, fn: (l: Leaf) => Pane): Pane {
  return "children" in p
    ? {
        ...p,
        children: p.children.map((c) => update(c, id, fn)) as [Pane, Pane],
      }
    : p.id === id
      ? fn(p)
      : p;
}
export function removePane(p: Pane, id: string): Pane | null {
  if (p.id === id) return null;
  if (!("children" in p)) return p;
  const a = removePane(p.children[0], id),
    b = removePane(p.children[1], id);
  return a && b ? { ...p, children: [a, b] } : a || b;
}
