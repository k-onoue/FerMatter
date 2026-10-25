// @vitest-environment jsdom
import { useState } from "react";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { VariantEditor } from "./VariantEditor";
import { savedVariant } from "./variants";
import type { Card, Project } from "./types";

vi.mock("./api", () => ({
  asset: async (path: string) => "data:image/svg+xml," + path,
}));
vi.mock("./editor", () => ({
  CodeEditor: ({ value }: { value: string }) => (
    <textarea aria-label="Variant source" value={value} readOnly />
  ),
}));
beforeEach(() => localStorage.clear());
afterEach(cleanup);

function openVariant() {
  const source = {
    id: "source",
    name: "Synthetic",
    tex: "x^2",
    kind: "inline",
    origin: "source",
    file: "main.typ",
    sourceOrder: 0,
    status: "ready",
    svgPath: "source.svg",
    sourceFontSize: 11,
  } as Card;
  const changed = vi.fn(),
    saved = vi.fn(),
    closed = vi.fn(),
    dragged = vi.fn();
  function Harness() {
    const [editing, setEditing] = useState<Card>({
      ...source,
      id: "variant",
      origin: "custom",
      parentID: source.id,
    });
    return (
      <VariantEditor
        project={{ id: "p", main: "main.typ", format: "typst" } as Project}
        editing={editing}
        setEditing={(value) => {
          changed(value);
          setEditing(value);
        }}
        editPreview={source}
        editBusy={false}
        onSave={() => saved(savedVariant(editing, source))}
        onClose={closed}
        onDrag={dragged}
        onRender={() => {}}
      />
    );
  }
  render(<Harness />);
  return { source, changed, saved, closed, dragged };
}

it("changes a presentation variant's color and saves safely before its new preview", async () => {
  const { source, changed, saved, closed, dragged } = openVariant();
  const original = structuredClone(source);
  await screen.findByRole("img", { name: "x^2" });
  fireEvent.click(screen.getByRole("button", { name: "Equation color" }));
  fireEvent.click(screen.getByRole("button", { name: "Blue" }));
  expect(changed).toHaveBeenLastCalledWith(
    expect.objectContaining({
      instantColor: "2563eb",
      tex: "x^2",
      file: "main.typ",
      origin: "custom",
    }),
  );
  await waitFor(() => expect(screen.queryByRole("img")).toBeNull());
  fireEvent.dragStart(document.querySelector(".equation-image")!);
  expect(dragged).not.toHaveBeenCalled();
  fireEvent.keyDown(screen.getByRole("slider", { name: "Color" }), {
    key: "Escape",
  });
  expect(
    screen.queryByRole("dialog", { name: "Equation color picker" }),
  ).toBeNull();
  expect(screen.getByRole("dialog", { name: "Edit" })).toBeTruthy();
  expect(closed).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Save Variant" }));
  expect(saved).toHaveBeenCalledWith(
    expect.objectContaining({
      id: "variant",
      parentID: "source",
      instantColor: "2563eb",
      status: "pending",
    }),
  );
  expect(saved.mock.calls[0][0].svgPath).toBeUndefined();
  expect(source).toEqual(original);
});

it("allows arbitrary HEX and resets to the original source color without rewriting source", async () => {
  const { source, changed, saved } = openVariant();
  expect(
    (screen.getByRole("button", { name: "Reset color" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Equation color" }));
  const hex = screen.getByRole("textbox", { name: "Hex color" });
  fireEvent.change(hex, { target: { value: "#ABC123" } });
  expect(changed).toHaveBeenLastCalledWith(
    expect.objectContaining({ instantColor: "abc123" }),
  );
  fireEvent.keyDown(hex, { key: "Escape" });
  fireEvent.click(screen.getByRole("button", { name: "Reset color" }));
  expect(changed.mock.calls.at(-1)![0].instantColor).toBeUndefined();
  await screen.findByRole("img", { name: "x^2" });
  fireEvent.click(screen.getByRole("button", { name: "Save Variant" }));
  expect(saved.mock.calls[0][0].instantColor).toBeUndefined();
  expect(saved.mock.calls[0][0].svgPath).toBe("source.svg");
  expect(source.instantColor).toBeUndefined();
  expect(source.tex).toBe("x^2");
});
