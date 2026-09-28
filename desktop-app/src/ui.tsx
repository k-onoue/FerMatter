import { useEffect, useState } from "react";
import { X, LoaderCircle, TriangleAlert } from "lucide-react";
import { asset } from "./api";
import { cardOutputColor, isBrightColor } from "./outputColor";
import type { Card } from "./types";
export function ImagePreview({
  card,
  className = "",
  onDrag,
}: {
  card?: Card;
  className?: string;
  onDrag?: (card: Card) => void;
}) {
  const [src, setSrc] = useState("");
  const color = cardOutputColor(card?.instantColor);
  const bright = color && isBrightColor(color);
  useEffect(() => {
    let active = true;
    setSrc("");
    if (card?.svgPath)
      asset(card.svgPath)
        .then((s) => active && setSrc(s))
        .catch(() => {});
    return () => {
      active = false;
    };
  }, [card?.svgPath]);
  return (
    <div
      className={`equation-image ${card?.kind === "figure" ? "figure" : "math"} ${color ? "colored" : ""} ${bright ? "bright-ink" : ""} ${className}`}
      draggable={!!onDrag && !!src}
      onDragStart={(e) => {
        e.preventDefault();
        if (card && src) onDrag?.(card);
      }}
    >
      {src ? (
        <img src={src} draggable={false} alt={card?.tex || "Equation"} />
      ) : card?.status === "failed" ? (
        <TriangleAlert size={22} />
      ) : card?.status === "pending" ? (
        <LoaderCircle size={22} className="spin" />
      ) : (
        <span className="placeholder-equation">x² + y²</span>
      )}
    </div>
  );
}
export function Modal({
  title,
  children,
  onClose,
  wide = false,
}: {
  title: string;
  children: React.ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);
  return (
    <div
      className="modal-shade"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`modal ${wide ? "wide" : ""}`}
      >
        <header>
          <h2>{title}</h2>
          <button aria-label="Close" onClick={onClose}>
            <X size={19} />
          </button>
        </header>
        {children}
      </section>
    </div>
  );
}
export type Ask = (
  title: string,
  value?: string,
  label?: string,
) => Promise<string | null>;
export function Prompt({
  title,
  value,
  label,
  onDone,
}: {
  title: string;
  value: string;
  label: string;
  onDone: (value: string | null) => void;
}) {
  const [text, setText] = useState(value);
  return (
    <Modal title={title} onClose={() => onDone(null)}>
      <form
        className="prompt"
        onSubmit={(e) => {
          e.preventDefault();
          onDone(text);
        }}
      >
        <label>
          {label}
          <input
            autoFocus
            value={text}
            onChange={(e) => setText(e.target.value)}
            onFocus={(e) => e.target.select()}
          />
        </label>
        <footer>
          <button type="button" onClick={() => onDone(null)}>
            Cancel
          </button>
          <button className="primary" type="submit">
            Save
          </button>
        </footer>
      </form>
    </Modal>
  );
}
export function ContextMenu({
  x,
  y,
  items,
  onClose,
}: {
  x: number;
  y: number;
  items: { label: string; action: () => void; disabled?: boolean }[];
  onClose: () => void;
}) {
  useEffect(() => {
    const close = () => onClose();
    window.addEventListener("pointerdown", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("blur", close);
    };
  }, [onClose]);
  return (
    <div
      className="context-menu"
      style={{
        left: Math.min(x, innerWidth - 230),
        top: Math.min(y, innerHeight - items.length * 31 - 16),
      }}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {items.map((item, i) => (
        <button
          key={i}
          disabled={item.disabled}
          onClick={() => {
            onClose();
            item.action();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
