import { useCallback, useRef } from "react";
import { listenForControlScroll } from "./scrollZoom";

export function useControlScroll(
  onStep: (steps: number, event: WheelEvent) => void,
) {
  const callback = useRef(onStep);
  callback.current = onStep;
  // A callback ref also covers panes mounted after opening a project.
  return useCallback((element: HTMLElement | null) => {
    if (element)
      return listenForControlScroll(element, (steps, event) =>
        callback.current(steps, event),
      );
  }, []);
}
