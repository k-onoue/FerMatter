import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { CodeEditor } from "./editor";
import { Modal, type Ask } from "./ui";
import { rpc, uid } from "./api";
import {
  builtinTemplates,
  initialTemplates,
  composeProject,
  validateTemplate,
  type TemplateConfiguration,
  type InputMode,
} from "./templates";
import type { Project } from "./types";
export function NewProjectDialog({
  configuration,
  onConfiguration,
  onCreated,
  onClose,
  onError,
  ask,
  defaultDirectory,
}: {
  configuration: TemplateConfiguration;
  onConfiguration: (value: TemplateConfiguration) => void;
  onCreated: (project: Project) => void;
  onClose: () => void;
  onError: (error: string) => void;
  ask: Ask;
  defaultDirectory: string;
}) {
  const [name, setName] = useState("Untitled");
  const [id, setID] = useState(configuration.defaultID);
  const [mode, setMode] = useState<InputMode>("automatic");
  const [text, setText] = useState("x^2 + y^2 = r^2");
  const [editing, setEditing] = useState(false);
  const [templateDraft, setTemplateDraft] = useState("");
  const [compiler, setCompiler] = useState("pdflatex");
  const [creating, setCreating] = useState(false);
  const [directory, setDirectory] = useState("");
  const templates = [...builtinTemplates, ...configuration.custom];
  const template = templates.find((t) => t.id === id) || builtinTemplates[0];
  const fail = (e: unknown) => onError(String(e));
  async function saveConfiguration(config: TemplateConfiguration) {
    await rpc("templates", { value: config });
    onConfiguration(config);
  }
  async function saveTemplate() {
    try {
      const builtIn = template.id.startsWith("builtin-");
      const name = builtIn
        ? await ask("Save Template", template.name + " copy")
        : template.name;
      if (!name) return;
      const next = {
        ...template,
        id: builtIn ? uid() : template.id,
        name,
        source: templateDraft,
        compiler,
      };
      validateTemplate(next);
      await saveConfiguration({
        ...configuration,
        custom: [...configuration.custom.filter((t) => t.id !== next.id), next],
      });
      setID(next.id);
      setEditing(false);
    } catch (e) {
      fail(e);
    }
  }
  async function create() {
    try {
      setCreating(true);
      const content = composeProject(text, template, mode);
      onCreated(
        await rpc<Project>("new_project", {
          name,
          format: template.format,
          text: content,
          engine: template.compiler,
          directory,
        }),
      );
    } catch (e) {
      fail(e);
    } finally {
      setCreating(false);
    }
  }
  return (
    <Modal title="New Project" wide onClose={onClose}>
      <div className="variant-title">
        <input
          aria-label="Project name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <select
          aria-label="Template"
          value={template.id}
          onChange={(e) => {
            setID(e.target.value);
            setEditing(false);
          }}
        >
          {templates.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>
      </div>
      <div className="project-folder-row">
        <span>Location</span>
        <span className="folder-path" title={directory || defaultDirectory}>
          {directory || defaultDirectory}
        </span>
        <button
          onClick={() =>
            void open({
              directory: true,
              multiple: false,
              defaultPath: directory || defaultDirectory,
            })
              .then((path) => {
                if (typeof path === "string") setDirectory(path);
              })
              .catch(fail)
          }
        >
          Choose…
        </button>
        {directory && <button onClick={() => setDirectory("")}>Default</button>}
      </div>
      <div className="template-options">
        {editing && template.format === "latex" ? (
          <select
            aria-label="Template compiler"
            value={compiler}
            onChange={(e) => setCompiler(e.target.value)}
          >
            {["pdflatex", "xelatex", "lualatex"].map((c) => (
              <option key={c}>{c}</option>
            ))}
          </select>
        ) : (
          <select
            aria-label="Interpret pasted content"
            value={mode}
            onChange={(e) => setMode(e.target.value as InputMode)}
          >
            <option value="automatic">Auto</option>
            <option value="equations">One per line</option>
            <option value="source">Source</option>
          </select>
        )}
        <button
          disabled={configuration.defaultID === template.id}
          onClick={() =>
            void saveConfiguration({
              ...configuration,
              defaultID: template.id,
            }).catch(fail)
          }
        >
          Set as Default
        </button>
        {!template.id.startsWith("builtin-") && (
          <button
            onClick={() =>
              void saveConfiguration({
                custom: configuration.custom.filter(
                  (t) => t.id !== template.id,
                ),
                defaultID:
                  configuration.defaultID === template.id
                    ? initialTemplates.defaultID
                    : configuration.defaultID,
              })
                .then(() => {
                  setID(initialTemplates.defaultID);
                  setEditing(false);
                })
                .catch(fail)
            }
          >
            Delete Template
          </button>
        )}
      </div>
      <div className="new-editor">
        <CodeEditor
          project="new"
          path={template.id + (editing ? "-template" : "")}
          value={editing ? templateDraft : text}
          format={template.format}
          formatMode={!editing && mode === "source" ? "document" : false}
          onChange={editing ? setTemplateDraft : setText}
          onSave={() => void (editing ? saveTemplate() : create())}
        />
      </div>
      <footer>
        <button
          onClick={() => {
            if (editing) void saveTemplate();
            else {
              setTemplateDraft(template.source);
              setCompiler(template.compiler);
              setEditing(true);
            }
          }}
        >
          {editing ? "Save Template" : "Edit Template"}
        </button>
        <button
          onClick={() =>
            void open({
              multiple: false,
              filters: [{ name: "Source", extensions: ["tex", "typ", "txt"] }],
            })
              .then(async (path) => {
                if (typeof path === "string")
                  setText(await rpc<string>("load_source", { path }));
              })
              .catch(fail)
          }
        >
          Load File…
        </button>
        <span className="spacer" />
        <button onClick={onClose}>Cancel</button>
        <button
          className="primary"
          disabled={creating || editing}
          onClick={() => void create()}
        >
          {creating ? "Creating…" : "Create Project"}
        </button>
      </footer>
    </Modal>
  );
}
