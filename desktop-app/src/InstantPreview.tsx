import { useEffect, useRef, useState } from "react";
import { LoaderCircle } from "lucide-react";
import { prepareDrag } from "./artifactLease";
import type { InstantDraft, RenderedInstant } from "./instant";

export function InstantPreview({
  rendered,
  current,
  draft,
  busy,
  compileError,
  onRender,
  onDragStarted,
  onError,
}: {
  rendered: RenderedInstant | null;
  current: RenderedInstant | null;
  draft: InstantDraft;
  busy: boolean;
  compileError: string;
  onRender: () => void;
  onDragStarted: () => void;
  onError: (error: unknown) => void;
}) {
  const [dragGeneration, setDragGeneration] = useState(0);
  const [preparedDrag, setPreparedDrag] = useState<{
    result: RenderedInstant;
    start: () => Promise<void>;
  } | null>(null);
  const dragPointer = useRef<{
    id: number;
    x: number;
    y: number;
    start: () => Promise<void>;
  } | null>(null);
  const beginDrag =
    preparedDrag?.result === current ? preparedDrag?.start : null;
  useEffect(() => {
    setPreparedDrag(null);
    if (!current?.card.pngPath) return;
    let stopped = false;
    const prepared = prepareDrag(
      { item: [current.path], icon: current.card.pngPath, mode: "copy" },
      () => {
        if (!stopped) setDragGeneration((value) => value + 1);
      },
    );
    void prepared.acquired
      .then(() => {
        if (!stopped)
          setPreparedDrag({ result: current, start: prepared.start });
      })
      .catch(onError);
    return () => {
      stopped = true;
      prepared.dispose();
    };
  }, [current, dragGeneration, onError]);

  function releaseDragPointer(element: HTMLImageElement) {
    const pointer = dragPointer.current;
    dragPointer.current = null;
    if (pointer && element.hasPointerCapture?.(pointer.id))
      element.releasePointerCapture(pointer.id);
  }

  function drag() {
    if (!beginDrag) return;
    const started = beginDrag();
    setPreparedDrag(null);
    void started.catch(onError);
    onDragStarted();
  }

  return (
    <section
      className={`instant-preview ${draft.color.toLowerCase() === "#ffffff" ? "white-ink" : ""}`}
      aria-label="Equation preview"
      tabIndex={0}
    >
      {rendered ? (
        <img
          src={rendered.image}
          alt="Rendered equation"
          draggable={false}
          style={{ cursor: beginDrag ? "grab" : undefined }}
          onDragStart={(event) => event.preventDefault()}
          onPointerDown={(event) => {
            if (event.button !== 0 || !beginDrag) return;
            event.preventDefault();
            dragPointer.current = {
              id: event.pointerId,
              x: event.clientX,
              y: event.clientY,
              start: beginDrag,
            };
            event.currentTarget.setPointerCapture(event.pointerId);
          }}
          onPointerMove={(event) => {
            const pointer = dragPointer.current;
            if (!pointer || pointer.id !== event.pointerId) return;
            if (!(event.buttons & 1) || pointer.start !== beginDrag) {
              releaseDragPointer(event.currentTarget);
              return;
            }
            if (
              Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) <
              5
            )
              return;
            releaseDragPointer(event.currentTarget);
            drag();
          }}
          onPointerUp={(event) => releaseDragPointer(event.currentTarget)}
          onPointerCancel={(event) => releaseDragPointer(event.currentTarget)}
          onLostPointerCapture={() => {
            dragPointer.current = null;
          }}
        />
      ) : (
        <span className="instant-empty">Preview</span>
      )}
      <div className="instant-status" role="status">
        {draft.requiresRender ? (
          <button onClick={onRender}>Render</button>
        ) : busy ? (
          <>
            <LoaderCircle size={14} className="spin" />
            Rendering…
          </>
        ) : compileError ? (
          "Needs correction"
        ) : !current && draft.source.trim() ? (
          "Preparing…"
        ) : (
          ""
        )}
      </div>
    </section>
  );
}
