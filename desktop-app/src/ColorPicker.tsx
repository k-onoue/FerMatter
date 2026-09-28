import { useEffect, useId, useRef, useState } from "react";

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
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const id = useId();
  useEffect(() => setHex(value), [value]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [open]);

  return (
    <div
      ref={root}
      className="instant-color"
      onKeyDown={(event) => {
        if (event.key !== "Escape" || !open) return;
        event.preventDefault();
        event.stopPropagation();
        setOpen(false);
        setHex(value);
        trigger.current?.focus();
      }}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
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
          setHex(value);
          setOpen(!open);
        }}
      >
        <span style={{ backgroundColor: value }} />
      </button>
      {open && (
        <div
          id={id}
          role="dialog"
          aria-label="Equation color picker"
          className="instant-color-popover"
        >
          <div className="instant-color-swatches">
            {colors.map(([name, color]) => (
              <button
                key={color}
                aria-label={name}
                title={name}
                aria-pressed={value.toLowerCase() === color}
                onClick={() => onChange(color)}
              >
                <span style={{ backgroundColor: color }} />
              </button>
            ))}
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
                if (/^#?[0-9a-f]{6}$/i.test(text))
                  onChange("#" + text.replace(/^#/, "").toLowerCase());
              }}
              onBlur={() => setHex(value)}
            />
          </label>
        </div>
      )}
    </div>
  );
}
