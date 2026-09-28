import { useEffect, useLayoutEffect, useRef, useState } from "react";
import * as pdfjs from "pdfjs-dist";
import workerURL from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import {
  ChevronLeft,
  ChevronRight,
  Minus,
  Plus,
  RefreshCw,
  X,
  Maximize,
} from "lucide-react";
import { asset, rpc } from "./api";
import type { DocumentPDF, Anchor } from "./types";
import { anchorAtPoint } from "./navigation";
import { stepZoom } from "./scrollZoom";
import { useControlScroll } from "./useControlScroll";
pdfjs.GlobalWorkerOptions.workerSrc = workerURL;
export function PdfPane({
  document,
  building,
  onBuild,
  onClose,
  onNavigate,
  onSource,
  target,
}: {
  document: DocumentPDF | null;
  building: boolean;
  onBuild: () => void;
  onClose: () => void;
  onNavigate: (anchor: Anchor) => void;
  onSource: (file: string, line: number) => void;
  target?: string;
}) {
  const [pdf, setPdf] = useState<pdfjs.PDFDocumentProxy | null>(null);
  const [page, setPage] = useState(1);
  const [{ zoom, fit }, setView] = useState({ zoom: 1, fit: true });
  const [width, setWidth] = useState(400);
  const [error, setError] = useState("");
  const [pageSize, setPageSize] = useState({ width: 1, height: 1, scale: 1 });
  const canvas = useRef<HTMLCanvasElement>(null);
  const pageElement = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const zoomPoint = useRef<{
    x: number;
    y: number;
    clientX: number;
    clientY: number;
  } | null>(null);
  const navigation = useRef(0);
  const highlight = useRef<HTMLDivElement>(null);
  const selectedAnchor = document?.anchors.find(
    (a) => a.targetID === target && a.page === page,
  );
  const selectedName =
    document?.targets?.find((t) => t.id === target)?.name || "Equation";
  function zoomBy(steps: number, event?: WheelEvent) {
    if (
      event &&
      stepZoom(fit ? pageSize.scale : zoom, steps, 0.2, 4) ===
        (fit ? pageSize.scale : zoom)
    )
      return;
    const rect = pageElement.current?.getBoundingClientRect();
    zoomPoint.current =
      event && rect
        ? {
            x: (event.clientX - rect.left) / pageSize.scale,
            y: (event.clientY - rect.top) / pageSize.scale,
            clientX: event.clientX,
            clientY: event.clientY,
          }
        : null;
    setView((view) => ({
      fit: false,
      zoom: stepZoom(view.fit ? pageSize.scale : view.zoom, steps, 0.2, 4),
    }));
  }
  const scrollZoom = useControlScroll((steps, event) => {
    if (pdf && content.current?.contains(event.target as Node))
      zoomBy(steps, event);
    else if (pdf) zoomBy(steps);
  });
  useEffect(() => {
    const observer = new ResizeObserver((entries) =>
      setWidth(entries[0].contentRect.width),
    );
    if (content.current) observer.observe(content.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    let gone = false;
    let task: pdfjs.PDFDocumentLoadingTask | undefined;
    setError("");
    setPdf(null);
    navigation.current++;
    if (document)
      asset(document.pdfPath)
        .then((data) => {
          if (gone) return;
          const bytes = Uint8Array.from(atob(data.split(",")[1]), (c) =>
            c.charCodeAt(0),
          );
          const vendor = new URL("/vendor/pdfjs/", window.location.href).href;
          task = pdfjs.getDocument({
            data: bytes,
            useSystemFonts: true,
            cMapUrl: vendor + "cmaps/",
            cMapPacked: true,
            standardFontDataUrl: vendor + "standard_fonts/",
            wasmUrl: vendor + "wasm/",
          });
          return task.promise;
        })
        .then((p) => {
          if (p && !gone) {
            setPdf(p);
            setPage(1);
          }
        })
        .catch((e) => !gone && setError(String(e)));
    return () => {
      gone = true;
      navigation.current++;
      void task?.destroy();
    };
  }, [document?.pdfPath]);
  useEffect(() => {
    const a = document?.anchors.find((a) => a.targetID === target);
    if (a) setPage(a.page);
  }, [target, document, pdf]);
  useEffect(
    () => () => {
      navigation.current++;
      void rpc("cancel", { key: "pdf-reverse" });
    },
    [page, document?.pdfPath],
  );
  useEffect(() => {
    let gone = false;
    let render: pdfjs.RenderTask | undefined;
    if (pdf && canvas.current)
      pdf
        .getPage(Math.min(page, pdf.numPages))
        .then((p) => {
          if (gone) return;
          const natural = p.getViewport({ scale: 1 });
          const scale = fit
            ? Math.max(0.2, (width - 32) / natural.width)
            : zoom;
          const viewport = p.getViewport({ scale });
          setPageSize({
            width: viewport.width,
            height: viewport.height,
            scale,
          });
          const el = canvas.current!;
          el.width = Math.ceil(viewport.width * devicePixelRatio);
          el.height = Math.ceil(viewport.height * devicePixelRatio);
          el.style.width = viewport.width + "px";
          el.style.height = viewport.height + "px";
          render = p.render({
            canvas: el,
            viewport,
            transform: [devicePixelRatio, 0, 0, devicePixelRatio, 0, 0],
          });
          return render.promise;
        })
        .catch((e) => {
          if (e?.name !== "RenderingCancelledException" && !gone)
            setError(String(e));
        });
    return () => {
      gone = true;
      render?.cancel();
    };
  }, [pdf, page, zoom, fit, width]);
  useLayoutEffect(() => {
    const point = zoomPoint.current;
    const rect = pageElement.current?.getBoundingClientRect();
    if (point && rect && content.current) {
      content.current.scrollLeft +=
        rect.left + point.x * pageSize.scale - point.clientX;
      content.current.scrollTop +=
        rect.top + point.y * pageSize.scale - point.clientY;
      zoomPoint.current = null;
    } else {
      highlight.current?.scrollIntoView({
        block: "nearest",
        inline: "nearest",
      });
    }
  }, [target, pageSize, document?.anchors]);
  useEffect(() => {
    function command(e: Event) {
      const id = (e as CustomEvent<string>).detail;
      if (id === "nextPage")
        setPage((p) => Math.min(pdf?.numPages || 1, p + 1));
      if (id === "previousPage") setPage((p) => Math.max(1, p - 1));
      if (id === "fitPDF") setView((view) => ({ ...view, fit: true }));
      if (id === "zoomIn") zoomBy(1);
      if (id === "zoomOut") zoomBy(-1);
    }
    window.addEventListener("pdf-command", command);
    return () => window.removeEventListener("pdf-command", command);
  }, [pdf, pageSize.scale]);
  return (
    <section className="pdf-pane" ref={scrollZoom}>
      <header>
        <strong>PDF</strong>
        <span className="spacer" />
        <button
          title="Previous page"
          onClick={() => setPage((p) => Math.max(1, p - 1))}
        >
          <ChevronLeft size={16} />
        </button>
        <span>
          {page} / {pdf?.numPages || "—"}
        </span>
        <button
          title="Next page"
          onClick={() => setPage((p) => Math.min(pdf?.numPages || 1, p + 1))}
        >
          <ChevronRight size={16} />
        </button>
        <button title="Rebuild PDF" disabled={building} onClick={onBuild}>
          <RefreshCw size={16} className={building ? "spin" : ""} />
        </button>
        <button title="Close PDF" onClick={onClose}>
          <X size={16} />
        </button>
      </header>
      <div className="pdf-content" ref={content}>
        {error && <div className="error-box">{error}</div>}
        {!pdf && !error && (
          <div className="empty">
            <button className="primary" disabled={building} onClick={onBuild}>
              {building ? "Building…" : "Build PDF"}
            </button>
          </div>
        )}
        <div
          className="pdf-page"
          ref={pageElement}
          style={{
            display: pdf ? "block" : "none",
            width: pageSize.width,
            height: pageSize.height,
          }}
        >
          <canvas
            aria-label={`PDF page ${page}`}
            ref={canvas}
            onClick={async (e) => {
              if (!pdf || !document) return;
              setError("");
              const sequence = ++navigation.current;
              const rect = e.currentTarget.getBoundingClientRect();
              const p = await pdf.getPage(page);
              if (sequence !== navigation.current) return;
              const viewport = p.getViewport({ scale: 1 });
              const x = ((e.clientX - rect.left) * viewport.width) / rect.width,
                y = ((e.clientY - rect.top) * viewport.height) / rect.height;
              if (document.format === "latex") {
                try {
                  const result = await rpc<{ file: string; line: number }>(
                    "pdf_navigate",
                    { key: "pdf-reverse", document, reverse: true, page, x, y },
                  );
                  if (sequence === navigation.current)
                    onSource(result.file, result.line);
                } catch (error) {
                  if (sequence === navigation.current) setError(String(error));
                }
                return;
              }
              const nearby = anchorAtPoint(document.anchors, page, x, y);
              if (nearby) onNavigate(nearby);
            }}
          />
          {selectedAnchor && (
            <div
              ref={highlight}
              className="pdf-highlight"
              role="status"
              aria-label={`Selected equation: ${selectedName}`}
              style={{
                left: Math.max(0, selectedAnchor.x - 6) * pageSize.scale,
                top:
                  Math.max(
                    0,
                    selectedAnchor.y - (selectedAnchor.height ? 3 : 21),
                  ) * pageSize.scale,
                width:
                  Math.min(
                    (selectedAnchor.width || 100) + 12,
                    pageSize.width / pageSize.scale -
                      Math.max(0, selectedAnchor.x - 6),
                  ) * pageSize.scale,
                height: ((selectedAnchor.height || 36) + 6) * pageSize.scale,
              }}
            />
          )}
        </div>
      </div>
      <footer>
        <button title="Zoom out" onClick={() => zoomBy(-1)}>
          <Minus size={15} />
        </button>
        <span>{fit ? "Fit" : Math.round(zoom * 100) + "%"}</span>
        <button title="Zoom in" onClick={() => zoomBy(1)}>
          <Plus size={15} />
        </button>
        <button
          title="Fit width"
          onClick={() => setView((view) => ({ ...view, fit: true }))}
        >
          <Maximize size={15} />
        </button>
        {building && <span>Building…</span>}
      </footer>
    </section>
  );
}
