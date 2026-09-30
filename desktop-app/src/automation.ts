import type { Project } from "./types";

/** Add unseen API variants without replacing source buffers, draft edits, or deletions. */
export function mergeAutomationVariants(
  local: Project,
  saved: Project,
): Project {
  if (local.id !== saved.id) return local;
  const seen = Number(local.automationGeneration || 0);
  const generation = Number(saved.automationGeneration || 0);
  if (generation <= seen) return local;
  const existing = new Set(local.cards.map((card) => card.id));
  const additions = saved.cards.filter(
    (card) =>
      Number(card.automationGeneration || 0) > seen && !existing.has(card.id),
  );
  return {
    ...local,
    automationGeneration: generation,
    cards: [...local.cards, ...additions],
  };
}
