import { useEffect, useState } from "react";
import { FolderOpen, RotateCcw } from "lucide-react";
import { rpc } from "./api";

type Configuration = {
  schemaVersion: 1;
  kind: "mathlidelux.tools";
  paths: Record<string, string>;
};
type Tool = { name: string; group: string; path?: string; error?: string };
type ToolStatus = { configuration: Configuration; tools: Tool[] };

function sameConfiguration(a: Configuration | null, b: Configuration | null) {
  if (!a || !b) return a === b;
  return [...new Set([...Object.keys(a.paths), ...Object.keys(b.paths)])].every(
    (name) => (a.paths[name] || "") === (b.paths[name] || ""),
  );
}

export function useCompilerTools() {
  const [saved, setSaved] = useState<Configuration | null>(null);
  const [draft, setDraft] = useState<Configuration | null>(null);
  const [checked, setChecked] = useState<Configuration | null>(null);
  const [tools, setTools] = useState<Tool[]>([]);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState("");
  function accept(result: ToolStatus) {
    setSaved(result.configuration);
    setDraft(result.configuration);
    setChecked(result.configuration);
    setTools(result.tools);
  }
  useEffect(() => {
    let active = true;
    void rpc<ToolStatus>("tools")
      .then((result) => {
        if (active) accept(result);
      })
      .catch((e) => {
        if (active) setError(String(e));
      })
      .finally(() => {
        if (active) setBusy(false);
      });
    return () => {
      active = false;
    };
  }, []);
  const dirty = !sameConfiguration(saved, draft);
  function change(name: string, path: string) {
    if (!draft) return;
    const paths = { ...draft.paths };
    if (path) paths[name] = path;
    else delete paths[name];
    setDraft({ ...draft, paths });
    setError("");
  }
  async function request(
    operation: "save_tools" | "reload_tools" | "check_tools",
  ) {
    setBusy(true);
    setError("");
    try {
      const result = await rpc<ToolStatus>(operation, { value: draft });
      if (operation === "check_tools") {
        setTools(result.tools);
        setChecked(result.configuration);
      } else accept(result);
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      setBusy(false);
    }
  }
  return {
    draft,
    checked,
    tools,
    busy,
    dirty,
    error,
    change,
    setError,
    save: () => request("save_tools"),
    reload: () => request("reload_tools"),
    check: () => request("check_tools"),
  };
}

export function CompilerToolFields({
  state,
  disabled = false,
}: {
  state: ReturnType<typeof useCompilerTools>;
  disabled?: boolean;
}) {
  const locked = state.busy || disabled;
  function rows(group: string) {
    return state.tools
      .filter((tool) => tool.group === group)
      .map((tool) => {
        const value = state.draft?.paths[tool.name] || "";
        const checked = sameConfiguration(state.draft, state.checked);
        return (
          <div className="compiler-tool" key={tool.name}>
            <label htmlFor={`tool-${tool.name}`}>{tool.name}</label>
            <div className="compiler-path">
              <input
                id={`tool-${tool.name}`}
                value={value}
                placeholder={(checked && tool.path) || "Automatic"}
                title={value || (checked && tool.path) || "Automatic"}
                spellCheck={false}
                autoCapitalize="off"
                disabled={locked}
                onChange={(e) => state.change(tool.name, e.target.value)}
              />
              <button
                title={`Choose ${tool.name} executable`}
                aria-label={`Choose ${tool.name} executable`}
                disabled={locked}
                onClick={() =>
                  void rpc<string | null>("choose_tool", {
                    name: tool.name,
                    path: value || tool.path || "",
                  })
                    .then((path) => {
                      if (typeof path === "string")
                        state.change(tool.name, path);
                    })
                    .catch((e) => state.setError(String(e)))
                }
              >
                <FolderOpen size={16} />
              </button>
              <button
                title={`Use automatic detection for ${tool.name}`}
                aria-label={`Use automatic detection for ${tool.name}`}
                disabled={locked || !value}
                onClick={() => state.change(tool.name, "")}
              >
                <RotateCcw size={14} />
              </button>
            </div>
            {checked && tool.error && <small>{tool.error}</small>}
          </div>
        );
      });
  }
  return (
    <div className="compiler-tools">
      <p className="muted">Leave paths empty to detect automatically.</p>
      {rows("Compilers")}
      <details>
        <summary>Additional tools</summary>
        {rows("Additional tools")}
      </details>
      {state.error && (
        <p role="alert" className="setup-error">
          {state.error}
        </p>
      )}
    </div>
  );
}

export function CompilerTools() {
  const state = useCompilerTools();
  return (
    <>
      <h3>Compiler tools</h3>
      <CompilerToolFields state={state} />
      <div className="compiler-actions">
        <button
          disabled={state.busy || !state.draft}
          onClick={() => void state.check()}
        >
          Check paths
        </button>
        <button disabled={state.busy} onClick={() => void state.reload()}>
          Reload
        </button>
        <button
          className="primary"
          disabled={state.busy || !state.dirty}
          onClick={() => void state.save()}
        >
          Apply
        </button>
      </div>
    </>
  );
}
