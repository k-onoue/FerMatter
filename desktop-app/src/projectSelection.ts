export function projectSelection(
  ids: string[],
  selected: Set<string>,
  id: string,
  anchor: string | undefined,
  modifiers: { shiftKey: boolean; metaKey: boolean; ctrlKey: boolean },
) {
  if (modifiers.shiftKey && anchor && ids.includes(anchor)) {
    const a = ids.indexOf(anchor),
      b = ids.indexOf(id);
    const range = ids.slice(Math.min(a, b), Math.max(a, b) + 1);
    return new Set(
      modifiers.metaKey || modifiers.ctrlKey ? [...selected, ...range] : range,
    );
  }
  if (modifiers.metaKey || modifiers.ctrlKey) {
    const next = new Set(selected);
    next.has(id) ? next.delete(id) : next.add(id);
    return next;
  }
  return new Set([id]);
}
