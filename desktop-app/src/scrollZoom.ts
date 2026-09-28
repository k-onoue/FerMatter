// Keep smooth trackpads and notched wheels on the same 10% zoom steps.
export function listenForControlScroll(
  target: EventTarget,
  onStep: (steps: number, event: WheelEvent) => void,
) {
  let accumulated = 0;
  let lastTime = 0;
  const wheel = (input: Event) => {
    const event = input as WheelEvent;
    if (
      !event.ctrlKey ||
      event.metaKey ||
      event.altKey ||
      event.shiftKey ||
      !event.deltaY ||
      Math.abs(event.deltaX) > Math.abs(event.deltaY)
    ) {
      accumulated = 0;
      return;
    }
    // React's wheel listeners are passive. Use a native listener so the pane
    // doesn't scroll or trigger WebKit page zoom while changing its own scale.
    event.preventDefault();
    event.stopPropagation();
    const delta =
      event.deltaY *
      (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 100 : 1);
    if (event.timeStamp - lastTime > 200 || delta * accumulated < 0)
      accumulated = 0;
    lastTime = event.timeStamp;
    accumulated += delta;
    const steps = Math.trunc(accumulated / 40);
    if (steps) {
      accumulated -= steps * 40;
      onStep(-Math.sign(steps) * Math.min(3, Math.abs(steps)), event);
    }
  };
  target.addEventListener("wheel", wheel, { passive: false });
  return () => target.removeEventListener("wheel", wheel);
}

export function stepZoom(zoom: number, steps: number, min = 0.5, max = 2) {
  return Math.max(
    min,
    Math.min(max, Math.round((zoom + steps * 0.1) * 100) / 100),
  );
}
