import { LoaderCircle, RotateCcw } from "lucide-react";
import { Modal, ImagePreview } from "./ui";
import { CodeEditor } from "./editor";
import { OutputSizeOptions } from "./OutputSizeOptions";
import { ColorPicker } from "./ColorPicker";
import { savedVariant } from "./variants";
import { cardOutputColor } from "./outputColor";
import type { Card, Project } from "./types";
export function VariantEditor({
  project,
  editing,
  setEditing,
  editPreview,
  editBusy,
  onClose,
  onSave,
  onDrag,
  onRender,
}: {
  project: Project;
  editing: Card;
  setEditing: (card: Card) => void;
  editPreview: Card | null;
  editBusy: boolean;
  onClose: () => void;
  onSave: () => void;
  onDrag: (card: Card) => void;
  onRender: () => void;
}) {
  const preview = savedVariant(editing, editPreview);
  const color = cardOutputColor(editing.instantColor) ?? "#000000";
  return (
    <Modal title="Edit Variant" onClose={onClose} wide>
      <div className="variant-title">
        <input
          aria-label="Variant name"
          value={editing.name}
          onChange={(e) => setEditing({ ...editing, name: e.target.value })}
        />
        <select
          aria-label="Equation type"
          value={editing.kind}
          onChange={(e) =>
            setEditing({ ...editing, kind: e.target.value as Card["kind"] })
          }
        >
          <option value="inline">Inline</option>
          <option value="block">Block</option>
          <option value="figure">Figure</option>
        </select>
      </div>
      <div className="variant-preview">
        {editing.requiresRender === true && (
          <button className="primary" onClick={onRender}>
            Render
          </button>
        )}
        {editBusy && <LoaderCircle size={15} className="spin" />}
        {editing.requiresRender !== true && (
          <ImagePreview card={preview} onDrag={onDrag} />
        )}
        {preview.error && <pre className="error-box">{preview.error}</pre>}
      </div>
      <div className="variant-editor">
        <CodeEditor
          project={project.id}
          focusRequest={editing.restoredContextFile ? editing.id : undefined}
          path={
            "library/" +
            editing.id +
            (project.format === "typst" ? ".typ" : ".tex")
          }
          value={editing.tex}
          format={project.format}
          formatMode={editing.kind === "figure" ? "document" : "math"}
          onChange={(tex) => setEditing({ ...editing, tex })}
          onSave={onSave}
        />
      </div>
      <footer>
        <label>
          Output size{" "}
          <select
            value={editing.exportFontSize ?? -1}
            onChange={(e) =>
              setEditing({
                ...editing,
                exportFontSize:
                  +e.target.value < 0 ? undefined : +e.target.value,
              })
            }
          >
            <OutputSizeOptions
              value={editing.exportFontSize ?? -1}
              sourceSizes={[editing.sourceFontSize]}
              projectSize={project.exportFontSize || 0}
            />
          </select>
        </label>
        <ColorPicker
          value={color}
          onChange={(color) =>
            setEditing({ ...editing, instantColor: color.slice(1) })
          }
        />
        <button
          aria-label="Reset color"
          title="Use source color"
          disabled={!editing.instantColor}
          onClick={() => setEditing({ ...editing, instantColor: undefined })}
        >
          <RotateCcw size={16} />
        </button>
        <span className="spacer" />
        <button onClick={onClose}>Cancel</button>
        <button className="primary" onClick={onSave}>
          Save Variant
        </button>
      </footer>
    </Modal>
  );
}
