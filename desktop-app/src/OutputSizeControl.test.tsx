// @vitest-environment jsdom
import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { OutputSizeControl } from "./OutputSizeControl";

afterEach(cleanup);
it("inherits changing defaults, accepts decimal sizes and resets to inheritance", () => {
  const changed = vi.fn();
  function Harness({ defaultSize }: { defaultSize: number }) {
    const [value, setValue] = useState<number>();
    return (
      <OutputSizeControl
        value={value}
        defaultSize={defaultSize}
        onChange={(n) => {
          setValue(n);
          changed(n);
        }}
      />
    );
  }
  const view = render(<Harness defaultSize={30} />);
  const input = screen.getByRole("spinbutton") as HTMLInputElement;
  expect(input.value).toBe("30");
  expect(screen.queryByRole("combobox")).toBeNull();
  view.rerender(<Harness defaultSize={40} />);
  expect(input.value).toBe("40");
  fireEvent.change(input, { target: { value: "" } });
  fireEvent.change(input, { target: { value: "40" } });
  fireEvent.blur(input);
  expect(changed).toHaveBeenLastCalledWith(40);
  expect(screen.getByRole("button").getAttribute("aria-pressed")).toBe("false");
  fireEvent.change(input, { target: { value: "31.25" } });
  fireEvent.keyDown(input, { key: "Enter" });
  expect(changed).toHaveBeenLastCalledWith(31.25);
  view.rerender(<Harness defaultSize={50} />);
  expect(input.value).toBe("31.25");
  fireEvent.click(
    screen.getByRole("button", { name: "Use default output font size" }),
  );
  expect(changed).toHaveBeenLastCalledWith(undefined);
  expect(input.value).toBe("50");
});

it("allows incomplete typing and rejects invalid sizes without clearing a saved override", () => {
  const changed = vi.fn();
  render(<OutputSizeControl value={24} onChange={changed} />);
  const input = screen.getByRole("spinbutton") as HTMLInputElement;
  for (const value of ["", "0", "-5", "201"]) {
    fireEvent.change(input, { target: { value } });
    expect(changed).not.toHaveBeenCalled();
    fireEvent.blur(input);
    expect(input.value).toBe("24");
  }
  fireEvent.change(input, { target: { value: "16.5" } });
  fireEvent.keyDown(input, { key: "Escape" });
  expect(input.value).toBe("24");
  expect(changed).not.toHaveBeenCalled();
  fireEvent.change(input, { target: { value: "16.5" } });
  fireEvent.blur(input);
  expect(changed).toHaveBeenLastCalledWith(16.5);
});

it("preserves legacy source-size choices without silently converting them to defaults", () => {
  const changed = vi.fn();
  const view = render(
    <OutputSizeControl value={0} sourceSizes={[10.95]} onChange={changed} />,
  );
  const input = screen.getByRole("spinbutton") as HTMLInputElement;
  expect(input.value).toBe("10.95");
  fireEvent.blur(input);
  expect(changed).not.toHaveBeenCalled();
  view.rerender(
    <OutputSizeControl value={0} sourceSizes={[10, 12]} onChange={changed} />,
  );
  expect(input.value).toBe("");
  fireEvent.blur(input);
  expect(changed).not.toHaveBeenCalled();
  expect(screen.getByRole("button").getAttribute("aria-pressed")).toBe("false");
});
