import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { HexColorPicker } from "react-colorful";
import { loadColorHistory, rememberColor } from "./colorHistory";
import "./ColorPicker.css";

const colors = [
  ["Black", "#000000"],
  ["White", "#ffffff"],
  ["Red", "#dc2626"],
  ["Orange", "#ea580c"],
  ["Yellow", "#eab308"],
  ["Green", "#16a34a"],
  ["Blue", "#2563eb"],
  ["Purple", "#9333ea"],
];

export function ColorPicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (color: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [hex, setHex] = useState(value);
  const [history, setHistory] = useState(loadColorHistory);
  const [position, setPosition] = useState({ left: 0, top: 0 });
  const root = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const hexEdited = useRef(false);
  const id = useId();
  const contains = (target: EventTarget | null) =>
    target instanceof Node &&
    (root.current?.contains(target) || panel.current?.contains(target));
  const resetFields = () => {
    setHex(value);
    hexEdited.current = false;
  };
  const remember = (color: string) => setHistory(rememberColor(color, history));
  const choose = (color: string) => {
    onChange(color);
    remember(color);
  };
  const rememberHex = () => {
    if (hexEdited.current && /^#?[0-9a-f]{6}$/i.test(hex))
      remember("#" + hex.replace(/^#/, "").toLowerCase());
    hexEdited.current = false;
  };
  const close = () => {
    rememberHex();
    setOpen(false);
  };
  useEffect(() => setHex(value), [value]);
  useLayoutEffect(() => {
    if (!open) return;
    const place = () => {
      if (!trigger.current || !panel.current) return;
      const anchor = trigger.current.getBoundingClientRect();
      const { width, height } = panel.current.getBoundingClientRect();
      const above = anchor.top - height - 10;
      setPosition({
        left: Math.max(8, Math.min(anchor.left, window.innerWidth - width - 8)),
        top: Math.max(
          8,
          Math.min(
            above >= 8 ? above : anchor.bottom + 10,
            window.innerHeight - height - 8,
          ),
        ),
      });
    };
    place();
    panel.current?.querySelector<HTMLElement>('[role="slider"]')?.focus();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(place);
    if (panel.current) observer?.observe(panel.current);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
      observer?.disconnect();
    };
  }, [open]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!contains(event.target)) close();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      close();
      resetFields();
      trigger.current?.focus();
    };
    const reloadHistory = () => setHistory(loadColorHistory());
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape, true);
    window.addEventListener("storage", reloadHistory);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape, true);
      window.removeEventListener("storage", reloadHistory);
    };
  }, [open, hex, history]);

  return (
    <div
      ref={root}
      className="instant-color"
      onBlur={(event) => {
        // WebKit may blur a slider without focusing the clicked swatch button.
        // Outside pointer events handle that case; only close on a known focus move.
        if (event.relatedTarget && !contains(event.relatedTarget)) close();
      }}
    >
      <button
        ref={trigger}
        className="instant-color-trigger"
        aria-label="Equation color"
        title="Equation color"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        onClick={() => {
          if (open) {
            close();
            return;
          }
          resetFields();
          setHistory(loadColorHistory());
          setOpen(true);
        }}
      >
        <span style={{ backgroundColor: value }} />
      </button>
      {open &&
        createPortal(
          <div
            ref={panel}
            id={id}
            role="dialog"
            aria-label="Equation color picker"
            className="instant-color-popover"
            style={position}
          >
            <div className="instant-color-heading">Default</div>
            <div
              className="instant-color-swatches"
              role="group"
              aria-label="Default colors"
            >
              {colors.map(([name, color]) => (
                <button
                  key={color}
                  aria-label={name}
                  title={name}
                  aria-pressed={value.toLowerCase() === color}
                  onClick={() => choose(color)}
                >
                  <span style={{ backgroundColor: color }} />
                </button>
              ))}
            </div>
            <div className="instant-color-heading">Color Picker</div>
            <HexColorPicker
              color={value}
              onChange={onChange}
              onChangeEnd={remember}
            />
            <div className="instant-color-heading">History</div>
            <div
              className="instant-color-swatches"
              role="group"
              aria-label="Color history"
            >
              {history.length ? (
                history.map((color) => (
                  <button
                    key={color}
                    aria-label={color}
                    title={color}
                    aria-pressed={value.toLowerCase() === color}
                    onClick={() => choose(color)}
                  >
                    <span style={{ backgroundColor: color }} />
                  </button>
                ))
              ) : (
                <span className="instant-color-empty">—</span>
              )}
            </div>
            <label className="instant-color-hex">
              HEX
              <input
                aria-label="Hex color"
                type="text"
                spellCheck={false}
                autoComplete="off"
                maxLength={7}
                value={hex}
                onChange={(event) => {
                  const text = event.target.value;
                  setHex(text);
                  hexEdited.current = true;
                  if (/^#?[0-9a-f]{6}$/i.test(text))
                    onChange("#" + text.replace(/^#/, "").toLowerCase());
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") rememberHex();
                }}
                onBlur={() => {
                  rememberHex();
                  setHex(value);
                }}
              />
            </label>
          </div>,
          document.body,
        )}
    </div>
  );
}
