import { expect, it, vi } from "vitest";
import { listenForControlScroll, stepZoom } from "./scrollZoom";

function wheel(
  target: EventTarget,
  deltaY: number,
  options: Partial<WheelEvent> = {},
) {
  const event = new Event("wheel", { cancelable: true });
  const properties = {
    ctrlKey: true,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    deltaX: 0,
    deltaY,
    deltaMode: 0,
    timeStamp: 100,
    ...options,
  };
  Object.defineProperties(
    event,
    Object.fromEntries(
      Object.entries(properties).map(([key, value]) => [key, { value }]),
    ),
  );
  target.dispatchEvent(event);
  return event;
}

it("routes Ctrl-scroll only to the hovered pane and starts PDF zoom at its fitted scale", () => {
  const projects = new EventTarget(),
    canvas = new EventTarget(),
    pdf = new EventTarget();
  let projectZoom = 1,
    canvasZoom = 1,
    pdfZoom = 0.63;
  listenForControlScroll(projects, (steps) => {
    projectZoom = stepZoom(projectZoom, steps);
  });
  listenForControlScroll(canvas, (steps) => {
    canvasZoom = stepZoom(canvasZoom, steps);
  });
  listenForControlScroll(pdf, (steps) => {
    pdfZoom = stepZoom(pdfZoom, steps, 0.2, 4);
  });
  expect(wheel(pdf, -40).defaultPrevented).toBe(true);
  expect([projectZoom, canvasZoom, pdfZoom]).toEqual([1, 1, 0.73]);
  wheel(canvas, 40);
  expect([projectZoom, canvasZoom, pdfZoom]).toEqual([1, 0.9, 0.73]);
  wheel(projects, -40);
  expect([projectZoom, canvasZoom, pdfZoom]).toEqual([1.1, 0.9, 0.73]);
});

it("preserves plain/horizontal scrolling and removes the non-passive listener on unmount", () => {
  const pane = new EventTarget();
  const add = vi.spyOn(pane, "addEventListener");
  const step = vi.fn();
  const stop = listenForControlScroll(pane, step);
  expect(add).toHaveBeenCalledWith("wheel", expect.any(Function), {
    passive: false,
  });
  for (const options of [
    { ctrlKey: false },
    { metaKey: true },
    { altKey: true },
    { shiftKey: true },
    { deltaX: 100 },
  ]) {
    expect(wheel(pane, -40, options).defaultPrevented).toBe(false);
  }
  expect(step).not.toHaveBeenCalled();
  stop();
  expect(wheel(pane, -40).defaultPrevented).toBe(false);
  expect(step).not.toHaveBeenCalled();
});

it("accumulates smooth input, normalizes wheel units and discards stale or reversed motion", () => {
  const pane = new EventTarget();
  const step = vi.fn();
  listenForControlScroll(pane, step);
  wheel(pane, -10);
  wheel(pane, -10);
  wheel(pane, -20);
  expect(step.mock.calls.map(([steps]) => steps)).toEqual([1]);
  wheel(pane, 3, { deltaMode: 1 });
  wheel(pane, -1, { deltaMode: 2 });
  wheel(pane, -1000);
  expect(step.mock.calls.map(([steps]) => steps)).toEqual([1, -1, 2, 3]);
  step.mockClear();
  wheel(pane, 30, { timeStamp: 500 });
  wheel(pane, -20, { timeStamp: 510 });
  wheel(pane, -30, { timeStamp: 900 });
  expect(step).not.toHaveBeenCalled();
  wheel(pane, -10, { timeStamp: 910 });
  expect(step.mock.calls.map(([steps]) => steps)).toEqual([1]);
});

it("consumes Ctrl-scroll at zoom limits without changing window zoom", () => {
  const pane = new EventTarget();
  let zoom = 2;
  listenForControlScroll(pane, (steps) => {
    zoom = stepZoom(zoom, steps);
  });
  expect(wheel(pane, -120).defaultPrevented).toBe(true);
  expect(zoom).toBe(2);
  expect(stepZoom(0.5, -3)).toBe(0.5);
  expect(stepZoom(3.9, 3, 0.2, 4)).toBe(4);
  expect(stepZoom(0.23, -1, 0.2, 4)).toBe(0.2);
});
