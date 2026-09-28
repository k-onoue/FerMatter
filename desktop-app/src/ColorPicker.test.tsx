// @vitest-environment jsdom
import { useState } from "react";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ColorPicker } from "./ColorPicker";
import { loadColorHistory, rememberColor } from "./colorHistory";

beforeEach(() => localStorage.clear());
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function picker(initial = "#000000") {
  const changed = vi.fn();
  function Harness() {
    const [color, setColor] = useState(initial);
    return (
      <ColorPicker
        value={color}
        onChange={(value) => {
          changed(value);
          setColor(value);
        }}
      />
    );
  }
  render(<Harness />);
  fireEvent.click(screen.getByRole("button", { name: "Equation color" }));
  return changed;
}

it("offers named color swatches and accepts a full HEX value without invoking a native picker", () => {
  const changed = picker();
  fireEvent.click(screen.getByRole("button", { name: "Red" }));
  expect(changed).toHaveBeenLastCalledWith("#dc2626");
  expect(
    screen.getByRole("button", { name: "Red" }).getAttribute("aria-pressed"),
  ).toBe("true");
  const hex = screen.getByRole("textbox", { name: "Hex color" });
  fireEvent.change(hex, { target: { value: "ABC123" } });
  expect(changed).toHaveBeenLastCalledWith("#abc123");
  expect((hex as HTMLInputElement).value).toBe("#abc123");
  expect(document.querySelector('input[type="color"]')).toBeNull();
});

function slider(name: string) {
  const control = screen.getByRole("slider", { name });
  vi.spyOn(control, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    left: 0,
    top: 0,
    right: 300,
    bottom: 100,
    width: 300,
    height: 100,
    toJSON() {},
  });
  return control;
}

it("drags through arbitrary hues and records only the final color", () => {
  const changed = picker("#ff0000");
  const hue = slider("Hue");
  fireEvent.mouseDown(hue, { clientX: 150, clientY: 50, buttons: 1 });
  expect(changed).toHaveBeenLastCalledWith("#00ffff");
  fireEvent.mouseMove(window, { clientX: 200, clientY: 50, buttons: 1 });
  expect(changed).toHaveBeenLastCalledWith("#0000ff");
  expect(loadColorHistory()).toEqual([]);
  fireEvent.mouseUp(window);
  expect(loadColorHistory()).toEqual(["#0000ff"]);
  const color = slider("Color");
  fireEvent.mouseDown(color, { clientX: 150, clientY: 25, buttons: 1 });
  expect(changed).toHaveBeenLastCalledWith("#6060bf");
  fireEvent.mouseUp(window);
  expect(loadColorHistory()).toEqual(["#6060bf", "#0000ff"]);
  expect(screen.getByRole("dialog")).toBeTruthy();
});

it("reaches white and black without losing the hue needed to return to color", () => {
  const changed = picker("#0000ff");
  const color = slider("Color");
  fireEvent.mouseDown(color, { clientX: 0, clientY: 0, buttons: 1 });
  fireEvent.mouseUp(window);
  expect(changed).toHaveBeenLastCalledWith("#ffffff");
  fireEvent.mouseDown(color, { clientX: 300, clientY: 100, buttons: 1 });
  fireEvent.mouseUp(window);
  expect(changed).toHaveBeenLastCalledWith("#000000");
  fireEvent.mouseDown(slider("Hue"), { clientX: 100, clientY: 50, buttons: 1 });
  fireEvent.mouseUp(window);
  fireEvent.mouseDown(color, { clientX: 300, clientY: 0, buttons: 1 });
  fireEvent.mouseUp(window);
  expect(changed).toHaveBeenLastCalledWith("#00ff00");
});

it("supports keyboard color adjustment and Enter to remember HEX input", () => {
  const changed = picker("#ff0000");
  expect(document.activeElement).toBe(
    screen.getByRole("slider", { name: "Color" }),
  );
  const hue = screen.getByRole("slider", { name: "Hue" });
  fireEvent.keyDown(hue, { key: "ArrowRight", keyCode: 39 });
  fireEvent.keyUp(hue, { key: "ArrowRight", keyCode: 39 });
  expect(changed).toHaveBeenLastCalledWith("#ff4d00");
  const hex = screen.getByRole("textbox", { name: "Hex color" });
  fireEvent.change(hex, { target: { value: "123ABC" } });
  expect(loadColorHistory()).toEqual(["#ff4d00"]);
  fireEvent.keyDown(hex, { key: "Enter" });
  expect(loadColorHistory()).toEqual(["#123abc", "#ff4d00"]);
  cleanup();
  picker();
  fireEvent.click(
    within(screen.getByRole("group", { name: "Color history" })).getByRole(
      "button",
      { name: "#123abc" },
    ),
  );
  expect(
    (screen.getByRole("textbox", { name: "Hex color" }) as HTMLInputElement)
      .value,
  ).toBe("#123abc");
});

it("keeps a bounded, deduplicated history and preserves choices from other windows", () => {
  picker();
  for (let i = 0; i < 20; i++)
    rememberColor("#" + i.toString(16).padStart(6, "0"), []);
  fireEvent.click(screen.getByRole("button", { name: "Red" }));
  expect(loadColorHistory()).toHaveLength(16);
  expect(loadColorHistory().slice(0, 3)).toEqual([
    "#dc2626",
    "#000013",
    "#000012",
  ]);
  fireEvent.click(screen.getByRole("button", { name: "Red" }));
  expect(
    loadColorHistory().filter((color) => color === "#dc2626"),
  ).toHaveLength(1);
  rememberColor("#abcdef", []);
  fireEvent(window, new StorageEvent("storage"));
  expect(
    within(screen.getByRole("group", { name: "Color history" }))
      .getAllByRole("button")[0]
      .getAttribute("aria-label"),
  ).toBe("#abcdef");
});

it("positions the portaled picker inside a narrow viewport", () => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
    function (this: HTMLElement) {
      const [left, top, width, height] =
        this.className === "instant-color-trigger"
          ? [430, 400, 32, 32]
          : [0, 0, 288, 390];
      return {
        x: left,
        y: top,
        left,
        top,
        width,
        height,
        right: left + width,
        bottom: top + height,
        toJSON() {},
      };
    },
  );
  vi.stubGlobal("innerWidth", 480);
  vi.stubGlobal("innerHeight", 460);
  try {
    picker();
    const dialog = screen.getByRole("dialog");
    expect(dialog.parentElement).toBe(document.body);
    expect(dialog.style.left).toBe("184px");
    expect(dialog.style.top).toBe("62px");
    vi.stubGlobal("innerHeight", 420);
    fireEvent.resize(window);
    expect(dialog.style.top).toBe("22px");
  } finally {
    vi.unstubAllGlobals();
  }
});

it("can still choose colors when history storage is unavailable", () => {
  vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
    throw new Error("Unavailable");
  });
  vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("Unavailable");
  });
  const changed = picker();
  fireEvent.click(screen.getByRole("button", { name: "Purple" }));
  expect(changed).toHaveBeenLastCalledWith("#9333ea");
  expect(
    within(screen.getByRole("group", { name: "Color history" })).getByRole(
      "button",
      { name: "#9333ea" },
    ),
  ).toBeTruthy();
});

it("does not save incomplete colors and closes by keyboard or outside click", () => {
  const changed = picker();
  const hex = screen.getByRole("textbox", { name: "Hex color" });
  fireEvent.change(hex, { target: { value: "#ff0" } });
  expect(changed).not.toHaveBeenCalled();
  fireEvent.blur(hex, { relatedTarget: document.body });
  fireEvent.click(screen.getByRole("button", { name: "Equation color" }));
  expect(
    (screen.getByRole("textbox", { name: "Hex color" }) as HTMLInputElement)
      .value,
  ).toBe("#000000");
  fireEvent.keyDown(screen.getByRole("textbox", { name: "Hex color" }), {
    key: "Escape",
  });
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(document.activeElement).toBe(
    screen.getByRole("button", { name: "Equation color" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Equation color" }));
  fireEvent.pointerDown(document.body);
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("keeps swatches available when WebKit blurs a slider without focusing the button", () => {
  const changed = picker();
  fireEvent.blur(screen.getByRole("slider", { name: "Color" }), {
    relatedTarget: null,
  });
  fireEvent.click(screen.getByRole("button", { name: "Blue" }));
  expect(changed).toHaveBeenLastCalledWith("#2563eb");
  expect(screen.getByRole("dialog")).toBeTruthy();
  fireEvent.keyDown(document.body, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(document.activeElement).toBe(
    screen.getByRole("button", { name: "Equation color" }),
  );
});
