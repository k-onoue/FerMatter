import { expect, it } from "vitest";
import { initialInstant, instantRevision, readyInstant } from "./instant";
import {
  commandForEvent,
  validateBindings,
  effectiveBindings,
} from "./shortcuts";
import type { Keybindings } from "./types";

it("only exports the current source, template, style, format and size", () => {
  const draft = { ...initialInstant(), source: "x^2" };
  const rendered = { revision: instantRevision(draft), path: "equation.svg" };
  expect(readyInstant(draft, rendered)).toBe(rendered);
  for (const change of [
    { source: "x^3" },
    { color: "#ffffff" },
    { fontSize: 48 },
    { kind: "inline" as const },
    { exportFormat: "pdf" },
    {
      template: {
        ...draft.template,
        source: draft.template.source + "% changed",
      },
    },
  ])
    expect(readyInstant({ ...draft, ...change }, rendered)).toBeNull();
  expect(readyInstant(draft, null)).toBeNull();
});

it("new Instant shortcut yields to a saved custom binding", () => {
  const old: Keybindings = {
    schemaVersion: 1,
    kind: "mathlidelux.keybindings",
    bindings: {
      open: {
        key: "k",
        command: true,
        shift: true,
        option: false,
        control: false,
      },
    },
  };
  expect(() => validateBindings(old)).not.toThrow();
  expect(effectiveBindings(old).instant.key).toBe("");
  expect(
    commandForEvent(
      {
        key: "k",
        metaKey: true,
        shiftKey: true,
        altKey: false,
        ctrlKey: false,
      } as KeyboardEvent,
      old,
    ),
  ).toBe("open");
});
