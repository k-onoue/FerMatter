import { useEffect, useRef, useState, type PointerEvent } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { GripVertical, LoaderCircle } from "lucide-react";
import { rpc } from "./api";
import { artworkAtPoint, artworkBounds } from "./navigation";
import type { Anchor, Card, DocumentPDF, Project } from "./types";

export function usePdfArtworkHover(
  document: DocumentPDF | null,
  page: number,
  scale: number,
  projectID: string,
) {
  const [hover, setHover] = useState<Anchor | null>(null);
  const cached = useRef<Anchor[]>([]);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const sequence = useRef(0);
  const pending = useRef(false);
  const key = `${projectID}:pdf-hover`;
  function cancel() {
    clearTimeout(timer.current);
    sequence.current++;
    if (pending.current) void rpc("cancel", { key });
    pending.current = false;
  }
  function leave() {
    cancel();
    setHover(null);
  }
  useEffect(() => {
    cached.current = [];
    setHover(null);
    return cancel;
  }, [document?.pdfPath, page, projectID]);
  function move(event: PointerEvent<HTMLDivElement>) {
    if (!document || event.buttons) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const x = (event.clientX - rect.left) / scale;
    const y = (event.clientY - rect.top) / scale;
    const anchor = artworkAtPoint(
      [...document.anchors, ...cached.current],
      document.targets,
      page,
      x,
      y,
      document.format,
    );
    cancel();
    setHover((old) =>
      old?.targetID === anchor?.targetID ? old : anchor || null,
    );
    if (anchor || document.format !== "latex") return;
    const current = sequence.current;
    timer.current = setTimeout(() => {
      pending.current = true;
      void rpc<Anchor | null>("pdf_drag_target", {
        key,
        document,
        reverse: true,
        page,
        x,
        y,
      })
        .then((anchor) => {
          if (sequence.current !== current || !anchor) return;
          cached.current = [
            ...cached.current.filter((a) => a.targetID !== anchor.targetID),
            anchor,
          ];
          setHover(anchor);
        })
        .catch(() => {}) // Prose and blank PDF space do not have drag targets.
        .finally(() => {
          if (sequence.current === current) pending.current = false;
        });
    }, 180);
  }
  return { hover, move, leave };
}

export function PdfArtworkDrag({
  anchor,
  document,
  project,
  scale,
  onNavigate,
}: {
  anchor: Anchor;
  document: DocumentPDF;
  project: Project;
  scale: number;
  onNavigate: (anchor: Anchor) => void;
}) {
  const [prepared, setPrepared] = useState<{ card: Card; path: string } | null>(
    null,
  );
  const [error, setError] = useState("");
  const dragged = useRef(false);
  const name =
    document.targets.find((t) => t.id === anchor.targetID)?.name || "Equation";
  const itemSize = project.cards.find(
    (c) => c.id === anchor.targetID,
  )?.exportFontSize;
  useEffect(() => {
    let gone = false;
    const key = `${project.id}:pdf-drag`;
    setPrepared(null);
    setError("");
    const timer = setTimeout(() => {
      void rpc<{ card: Card; path: string }>("pdf_prepare_drag", {
        key,
        project: project.id,
        document,
        targetID: anchor.targetID,
        format: project.exportFormat || "svg",
        fontSize: itemSize ?? project.exportFontSize ?? 0,
      })
        .then((result) => {
          if (!gone) setPrepared(result);
        })
        .catch((error) => {
          if (!gone && String(error) !== "Cancelled") setError(String(error));
        });
    }, 120);
    return () => {
      gone = true;
      clearTimeout(timer);
      void rpc("cancel", { key });
    };
  }, [
    anchor.targetID,
    document.pdfPath,
    project.id,
    project.exportFormat,
    project.exportFontSize,
    itemSize,
  ]);
  const bounds = artworkBounds(anchor);
  return (
    <button
      type="button"
      className={`pdf-artwork-drag ${prepared ? "ready" : ""}`}
      aria-label={
        prepared
          ? `Drag ${name}`
          : error
            ? `Cannot drag ${name}`
            : `Preparing ${name}`
      }
      title={error || (prepared ? `Drag ${name}` : `Preparing ${name}…`)}
      style={{
        left: bounds.x * scale,
        top: bounds.y * scale,
        width: bounds.width * scale,
        height: bounds.height * scale,
      }}
      draggable={!!prepared}
      onPointerDown={() => {
        dragged.current = false;
      }}
      onClick={(event) => {
        event.stopPropagation();
        if (!dragged.current) onNavigate(anchor);
      }}
      onDragStart={(event) => {
        event.preventDefault();
        event.stopPropagation();
        if (!prepared?.card.pngPath) return;
        dragged.current = true;
        void startDrag({
          item: [prepared.path],
          icon: prepared.card.pngPath,
          mode: "copy",
        }).catch((error) => setError(String(error)));
      }}
    >
      {prepared ? (
        <GripVertical size={14} />
      ) : (
        !error && <LoaderCircle size={14} className="spin" />
      )}
    </button>
  );
}
