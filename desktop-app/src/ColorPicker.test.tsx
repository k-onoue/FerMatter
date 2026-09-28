// @vitest-environment jsdom
import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ColorPicker } from "./ColorPicker";

afterEach(cleanup);

function picker() {
  const changed = vi.fn();
  function Harness() {
    const [color, setColor] = useState("#000000");
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

it("does not save incomplete colors and closes by keyboard or outside click", () => {
  const changed = picker();
  const hex = screen.getByRole("textbox", { name: "Hex color" });
  fireEvent.change(hex, { target: { value: "#ff0" } });
  expect(changed).not.toHaveBeenCalled();
  fireEvent.blur(hex);
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
