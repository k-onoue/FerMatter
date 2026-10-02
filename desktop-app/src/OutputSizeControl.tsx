import { useEffect, useState } from "react";
import { DEFAULT_OUTPUT_SIZE, outputSizeText } from "./outputSizes";

export function OutputSizeControl({
  value,
  defaultSize = DEFAULT_OUTPUT_SIZE,
  sourceSizes = [],
  onChange,
  label = "Output font size",
  allowDefault = true,
}: {
  value: number | undefined;
  defaultSize?: number;
  sourceSizes?: (number | undefined)[];
  onChange: (size: number | undefined) => void;
  label?: string;
  allowDefault?: boolean;
}) {
  const text = outputSizeText(value ?? defaultSize, sourceSizes);
  const [input, setInput] = useState(text);
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    setInput(text);
    setDirty(false);
  }, [text, value]);
  function commit() {
    if (!dirty) return;
    setDirty(false);
    const number = Number(input);
    if (input.trim() && Number.isFinite(number) && number >= 1 && number <= 200)
      onChange(number);
    else setInput(text);
  }
  return (
    <span className="output-size-control" role="group" aria-label={label}>
      {allowDefault && (
        <button
          type="button"
          aria-label={`Use default ${label.toLowerCase()}`}
          aria-pressed={value === undefined}
          title="Use default size"
          onClick={() => {
            setDirty(false);
            setInput(outputSizeText(defaultSize, sourceSizes));
            onChange(undefined);
          }}
        >
          Default
        </button>
      )}
      <input
        type="number"
        min="1"
        max="200"
        step="any"
        aria-label={label}
        value={input}
        placeholder="—"
        title={text ? `${text} pt` : "Mixed source sizes"}
        onChange={(e) => {
          setInput(e.target.value);
          setDirty(true);
        }}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            if (e.key === "Enter") commit();
            else {
              setInput(text);
              setDirty(false);
            }
          }
        }}
      />
      <span>pt</span>
    </span>
  );
}
