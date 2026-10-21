import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { HexColorPicker } from "react-colorful";
import { Pipette, LoaderCircle } from "lucide-react";
import { rpc } from "./api";
import { loadColorHistory, rememberColor } from "./colorHistory";
import { parseHexInput } from "./outputColor";
import "./ColorPicker.css";

const colors: [string, string][] = [
  ["Black", "#000000"],
  ["White", "#ffffff"],
  ["Red", "#dc2626"],
  ["Orange", "#ea580c"],
  ["Yellow", "#eab308"],
  ["Green", "#16a34a"],
  ["Blue", "#2563eb"],
  ["Purple", "#9333ea"],
];

function ColorSwatches({
  label,
  colors,
  value,
  onChoose,
}: {
  label: string;
  colors: [string, string][];
  value: string;
  onChoose: (color: string) => void;
}) {
  return (
    <div className="instant-color-swatches" role="group" aria-label={label}>
      {colors.length ? (
        colors.map(([name, color]) => (
          <button
            key={color}
            aria-label={name}
            title={name}
            aria-pressed={value.toLowerCase() === color}
            onClick={() => onChoose(color)}
          >
            <span style={{ backgroundColor: color }} />
          </button>
        ))
      ) : (
        <span className="instant-color-empty">—</span>
      )}
    </div>
  );
}

export function ColorPicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (color: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [sampling, setSampling] = useState(false);
  const [sampleError, setSampleError] = useState("");
  const sampleSequence = useRef(0);
  const valueRef = useRef(value);
  valueRef.current = value;
  const changeRef = useRef(onChange);
  changeRef.current = onChange;
  useEffect(
    () => () => {
      sampleSequence.current++;
    },
    [],
  );
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
    changeRef.current(color);
    remember(color);
  };
  const sample = async () => {
    if (sampling) return;
    const ticket = ++sampleSequence.current;
    const original = value;
    setSampling(true);
    setSampleError("");
    try {
      const picked = await rpc<string | null>("sample_color");
      if (sampleSequence.current !== ticket || valueRef.current !== original)
        return;
      const color = picked && parseHexInput(picked);
      if (color) choose(color);
    } catch (error) {
      if (sampleSequence.current === ticket) setSampleError(String(error));
    } finally {
      if (sampleSequence.current === ticket) setSampling(false);
    }
  };
  const rememberHex = () => {
    const color = parseHexInput(hex);
    if (hexEdited.current && color) remember(color);
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
            <ColorSwatches
              label="Default colors"
              colors={colors}
              value={value}
              onChoose={choose}
            />
            <div className="instant-color-heading instant-color-tools">
              Color Picker
              <button
                aria-label="Pick color from screen"
                title="Pick color from screen"
                disabled={sampling}
                onClick={() => void sample()}
              >
                {sampling ? (
                  <LoaderCircle size={16} className="spin" />
                ) : (
                  <Pipette size={16} />
                )}
              </button>
            </div>
            {sampleError && <small role="alert">{sampleError}</small>}
            <HexColorPicker
              color={value}
              onChange={onChange}
              onChangeEnd={remember}
            />
            <div className="instant-color-heading">History</div>
            <ColorSwatches
              label="Color history"
              colors={history.map((color) => [color, color])}
              value={value}
              onChoose={choose}
            />
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
                  const color = parseHexInput(text);
                  if (color) onChange(color);
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
