import { useEffect, useRef, useState } from "react";
import { AlignLeft, LoaderCircle, X } from "lucide-react";
import * as monaco from "monaco-editor";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import type { Palette } from "./types";
import { ModelOwnership } from "./modelOwnership";
import { registerWindowFlush, runWhenWindowInteractive } from "./persistence";
import { formatSource } from "./formatting";
import type { FormatMode } from "./formattingTypes";
(self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};
monaco.languages.register({ id: "latex" });
monaco.languages.register({ id: "typst" });
monaco.languages.setMonarchTokensProvider("latex", {
  tokenizer: {
    root: [
      [/%.*/, "comment"],
      [/\\[a-zA-Z@]+\*?/, "keyword"],
      [/[{}\[\]$&]/, "delimiter"],
      [/\d+(\.\d+)?/, "number"],
    ],
  },
});
monaco.languages.setMonarchTokensProvider("typst", {
  tokenizer: {
    root: [
      [/\/\/.*$/, "comment"],
      [/\/\*/, "comment", "@comment"],
      [/"([^"\\]|\\.)*"/, "string"],
      [/#[\w.-]+/, "keyword"],
      [
        /\b(?:let|set|show|import|include|as|context|if|else|for|in)\b/,
        "keyword",
      ],
      [/[\w.-]+(?=\()/, "keyword"],
      [/[{}\[\]$#()]/, "delimiter"],
      [/\b\d+(\.\d+)?(pt|em|cm|mm|%)?/, "number"],
    ],
    comment: [
      [/[^/*]+/, "comment"],
      [/\*\//, "comment", "@pop"],
      [/[/*]/, "comment"],
    ],
  },
});
for (const language of ["latex", "typst"])
  monaco.languages.setLanguageConfiguration(language, {
    comments:
      language === "latex"
        ? { lineComment: "%" }
        : { lineComment: "//", blockComment: ["/*", "*/"] },
    brackets: [
      ["{", "}"],
      ["[", "]"],
      ["(", ")"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: "$", close: "$" },
    ],
  });
const latex = [
  ["frac", "\\frac{${1:numerator}}{${2:denominator}}"],
  ["sqrt", "\\sqrt{${1:x}}"],
  ["sum", "\\sum_{${1:i=1}}^{${2:n}} ${3:x_i}"],
  ["int", "\\int_{${1:a}}^{${2:b}} ${3:f(x)}\\,dx"],
  ["begin", "\\begin{${1:align}}\n${2}\n\\end{${1:align}}"],
  ["text", "\\text{${1:text}}"],
  ["left", "\\left(${1:x}\\right)"],
  ...[
    "alpha",
    "beta",
    "gamma",
    "delta",
    "epsilon",
    "theta",
    "lambda",
    "mu",
    "pi",
    "sigma",
    "phi",
    "omega",
    "infty",
    "partial",
    "nabla",
    "times",
    "cdot",
    "leq",
    "geq",
    "mathbb",
    "mathbf",
    "usepackage",
    "newcommand",
    "section",
    "subsection",
    "input",
    "include",
    "label",
    "ref",
  ].map((x) => [x, `\\${x}`]),
];
const typst = [
  ["frac", "frac(${1:a}, ${2:b})"],
  ["sqrt", "sqrt(${1:x})"],
  ["sum", "sum_(${1:i=1})^${2:n} ${3:x_i}"],
  ["mat", "mat(${1:a}, ${2:b}; ${3:c}, ${4:d})"],
  ["vec", "vec(${1:x}, ${2:y})"],
  ["cases", 'cases(${1:x & "if" x > 0}, ${2:0 & "otherwise"})'],
  ...[
    "alpha",
    "beta",
    "gamma",
    "theta",
    "lambda",
    "pi",
    "sigma",
    "omega",
    "RR",
    "NN",
    "ZZ",
    "infinity",
    "dif",
    "integral",
    "norm",
    "include",
    "import",
    "let",
    "set",
    "show",
    "text",
    "equation",
    "canvas",
  ].map((x) => [x, x]),
];
const projectWords = new Map<string, string[]>();
export function setProjectCompletions(project: string, texts: string[]) {
  const words = new Set<string>();
  for (const text of texts) {
    for (const m of text.matchAll(
      /\\(?:newcommand|renewcommand|providecommand|DeclareMathOperator)\*?\s*\{?\\([A-Za-z@]+)|\\(?:def|gdef)\s*\\([A-Za-z@]+)|#let\s+([\w-]+)/g,
    )) {
      words.add(m[1] || m[2] || m[3]);
    }
  }
  projectWords.set(project, [...words]);
}
for (const language of ["latex", "typst"])
  monaco.languages.registerCompletionItemProvider(language, {
    triggerCharacters: language === "latex" ? ["\\"] : ["#", "."],
    provideCompletionItems(model, position) {
      const word = model.getWordUntilPosition(position);
      let start = word.startColumn;
      const before = model
        .getLineContent(position.lineNumber)
        .slice(0, start - 1);
      if (language === "latex" && before.endsWith("\\")) start--;
      const range = new monaco.Range(
        position.lineNumber,
        start,
        position.lineNumber,
        word.endColumn,
      );
      const project = model.uri.authority;
      const entries = [
        ...(language === "latex" ? latex : typst),
        ...(projectWords.get(project) || []).map((w) => [
          w,
          language === "latex" ? `\\${w}` : w,
        ]),
      ];
      return {
        suggestions: entries.map(([label, insertText], i) => ({
          label: language === "latex" ? `\\${label}` : label,
          insertText,
          kind: monaco.languages.CompletionItemKind.Function,
          insertTextRules:
            monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
          detail:
            i >= (language === "latex" ? latex.length : typst.length)
              ? "Project definition"
              : undefined,
        })),
      };
    },
  });
export function configureTheme(colors: Palette, dark: boolean) {
  monaco.editor.defineTheme("mathlidelux", {
    base: dark ? "vs-dark" : "vs",
    inherit: true,
    rules: [
      { token: "keyword", foreground: colors.command.slice(1) },
      { token: "delimiter", foreground: colors.delimiter.slice(1) },
      { token: "number", foreground: colors.number.slice(1) },
      { token: "comment", foreground: colors.comment.slice(1) },
      { token: "string", foreground: colors.delimiter.slice(1) },
    ],
    colors: {
      "editor.background": "#00000000",
      "editor.foreground": colors.text,
      "editorLineNumber.foreground": colors.comment,
      "editorLineNumber.activeForeground": colors.text,
      "editor.lineHighlightBackground": dark ? "#ffffff08" : "#00000005",
      "editor.selectionBackground": colors.accent + "45",
      "editorCursor.foreground": colors.accent,
      "editorWidget.background": colors.surfaceBackground,
      "editorSuggestWidget.background": colors.surfaceBackground,
      "editorSuggestWidget.foreground": colors.text,
      "editorSuggestWidget.selectedBackground": colors.accent + "40",
      "editorOverviewRuler.background": "#00000000",
      "scrollbar.shadow": "#00000000",
    },
  });
  monaco.editor.setTheme("mathlidelux");
}
const states = new Map<string, monaco.editor.ICodeEditorViewState>();
const ownership = new ModelOwnership<monaco.editor.ITextModel>((model, key) => {
  model.dispose();
  for (const state of states.keys())
    if (state.startsWith(key + "::")) states.delete(state);
});
export function disposeProjectModels(project: string) {
  ownership.releaseProject(project);
  projectWords.delete(project);
}
export interface EditorProps {
  project: string;
  path: string;
  modelKey?: string;
  retainModel?: boolean;
  readOnly?: boolean;
  value: string;
  format: string;
  pane?: string;
  onChange: (text: string) => void;
  onCaret?: (offset: number, line: number) => void;
  onFocus?: () => void;
  onSave?: () => void;
  reveal?: {
    line: number;
    nonce: number;
    startUTF16?: number;
    endUTF16?: number;
  };
  fontSize?: number;
  focusRequest?: string;
  formatMode?: FormatMode | false;
}
export function CodeEditor(props: EditorProps) {
  const container = useRef<HTMLDivElement>(null);
  const latest = useRef(props);
  latest.current = props;
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const formatting = useRef<AbortController | null>(null);
  const formatCurrent = useRef<() => Promise<void>>(async () => {});
  const [formatBusy, setFormatBusy] = useState(false);
  const [formatError, setFormatError] = useState("");
  const canFormat =
    props.formatMode !== false &&
    (props.format === "latex" || props.format === "typst");
  formatCurrent.current = async () => {
    const editor = editorRef.current;
    const model = editor?.getModel();
    const {
      format: language,
      formatMode: mode = "document",
      readOnly,
    } = latest.current;
    if (
      !editor ||
      !model ||
      readOnly ||
      mode === false ||
      (language !== "latex" && language !== "typst")
    )
      return;
    editor.focus();
    formatting.current?.abort();
    const request = new AbortController();
    formatting.current = request;
    const version = model.getVersionId();
    setFormatBusy(true);
    setFormatError("");
    try {
      const result = await formatSource(
        { source: model.getValue(), language, mode },
        request.signal,
      );
      if (
        request.signal.aborted ||
        editorRef.current !== editor ||
        model.isDisposed() ||
        editor.getModel() !== model ||
        model.getVersionId() !== version ||
        latest.current.readOnly ||
        document.documentElement.inert
      )
        return;
      formatting.current = null;
      if (result !== model.getValue()) {
        const viewState = editor.saveViewState();
        editor.pushUndoStop();
        editor.executeEdits("format-source", [
          { range: model.getFullModelRange(), text: result },
        ]);
        editor.pushUndoStop();
        if (viewState) editor.restoreViewState(viewState);
      }
    } catch (error) {
      if (!request.signal.aborted && editorRef.current === editor)
        setFormatError(
          (error instanceof Error ? error.message : String(error)).slice(
            0,
            240,
          ),
        );
    } finally {
      if (formatting.current === request || formatting.current === null) {
        formatting.current = null;
        setFormatBusy(false);
      }
    }
  };
  useEffect(() => {
    const node = container.current!;
    const uri = monaco.Uri.from({
      scheme: "mathlidelux",
      authority: props.project,
      path: "/" + (props.modelKey || props.path),
    });
    let model = monaco.editor.getModel(uri);
    if (!model) {
      model = monaco.editor.createModel(
        props.value,
        props.format === "typst"
          ? "typst"
          : props.path.endsWith(".json")
            ? "json"
            : props.format === "latex"
              ? "latex"
              : "plaintext",
        uri,
      );
    }
    const modelKey = uri.toString();
    const release = ownership.acquire(
      props.project,
      modelKey,
      model,
      !!props.retainModel,
    );
    const editor = monaco.editor.create(node, {
      model,
      theme: "mathlidelux",
      fontFamily: '"SFMono-Regular", Menlo, monospace',
      fontSize: props.fontSize || 14,
      lineHeight: 23,
      automaticLayout: true,
      editContext: false,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      wordWrap: "on",
      padding: { top: 16, bottom: 16 },
      lineNumbersMinChars: 3,
      renderLineHighlight: "line",
      smoothScrolling: true,
      tabSize: 2,
      insertSpaces: true,
      bracketPairColorization: { enabled: true },
      quickSuggestions: true,
      fixedOverflowWidgets: true,
      overviewRulerLanes: 0,
      hideCursorInOverviewRuler: true,
    });
    editorRef.current = editor;
    const key = modelKey + "::" + props.pane;
    const state = states.get(key);
    if (state) editor.restoreViewState(state);
    const changed = model.onDidChangeContent(() => {
      formatting.current?.abort();
      latest.current.onChange(model!.getValue());
    });
    const cursor = editor.onDidChangeCursorPosition((e) =>
      latest.current.onCaret?.(
        model!.getOffsetAt(e.position),
        e.position.lineNumber,
      ),
    );
    const focus = editor.onDidFocusEditorText(() => latest.current.onFocus?.());
    const cancelFormat = registerWindowFlush(`formatter:${key}`, async () => {
      formatting.current?.abort();
    });
    editor.addAction({
      id: "mathlidelux.formatSource",
      label: "Format Source",
      keybindings: [
        monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF,
      ],
      precondition: "!editorReadonly",
      run: () => formatCurrent.current(),
    });
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () =>
      latest.current.onSave?.(),
    );
    // Match AppKit's Cmd+Backspace behavior when already at the start of a line.
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Backspace, () => {
      const selection = editor.getSelection();
      if (!selection) return;
      let range: monaco.IRange = selection;
      if (selection.isEmpty()) {
        const pos = selection.getPosition();
        if (pos.column === 1 && pos.lineNumber > 1) {
          range = new monaco.Range(
            pos.lineNumber - 1,
            model!.getLineMaxColumn(pos.lineNumber - 1),
            pos.lineNumber,
            1,
          );
        } else {
          range = new monaco.Range(
            pos.lineNumber,
            1,
            pos.lineNumber,
            pos.column,
          );
        }
      }
      editor.pushUndoStop();
      editor.executeEdits("delete-to-line-start", [{ range, text: "" }]);
      editor.pushUndoStop();
    });
    return () => {
      formatting.current?.abort();
      cancelFormat();
      const viewState = editor.saveViewState();
      if (viewState) {
        states.delete(key);
        states.set(key, viewState);
        while (states.size > 256) states.delete(states.keys().next().value!);
      }
      changed.dispose();
      cursor.dispose();
      focus.dispose();
      editor.dispose();
      editorRef.current = null;
      release();
    };
  }, [props.project, props.modelKey || props.path, props.pane]);
  useEffect(() => {
    editorRef.current?.updateOptions({ readOnly: !!props.readOnly });
  }, [props.readOnly]);
  useEffect(() => {
    formatting.current?.abort();
    setFormatBusy(false);
    setFormatError("");
  }, [
    props.format,
    props.formatMode,
    props.readOnly,
    props.project,
    props.modelKey || props.path,
  ]);
  useEffect(() => {
    if (props.focusRequest)
      return runWhenWindowInteractive(() => editorRef.current?.focus());
  }, [props.focusRequest]);
  useEffect(() => {
    const editor = editorRef.current,
      model = editor?.getModel();
    if (model && model.getValue() !== props.value) {
      editor!.executeEdits("external-refresh", [
        { range: model.getFullModelRange(), text: props.value },
      ]);
      editor!.pushUndoStop();
    }
  }, [props.value]);
  useEffect(() => {
    if (props.reveal) {
      editorRef.current?.revealLineInCenter(props.reveal.line);
      editorRef.current?.setPosition({
        lineNumber: props.reveal.line,
        column: 1,
      });
      const model = editorRef.current?.getModel();
      if (
        model &&
        props.reveal.startUTF16 !== undefined &&
        props.reveal.endUTF16 !== undefined
      ) {
        const start = model.getPositionAt(props.reveal.startUTF16);
        const end = model.getPositionAt(props.reveal.endUTF16);
        editorRef.current?.setSelection(
          new monaco.Range(
            start.lineNumber,
            start.column,
            end.lineNumber,
            end.column,
          ),
        );
      }
      editorRef.current?.focus();
    }
  }, [props.reveal]);
  return (
    <div className="code-editor">
      {canFormat && (
        <div className="editor-format-toolbar">
          {formatError && (
            <span role="alert" className="editor-format-error">
              {formatError}
            </span>
          )}
          {formatError && (
            <button
              aria-label="Dismiss formatting error"
              onClick={() => setFormatError("")}
            >
              <X size={13} />
            </button>
          )}
          <span className="spacer" />
          <button
            disabled={props.readOnly || formatBusy}
            aria-label="Format Source"
            title={`Format ${props.format === "typst" ? "Typst" : "LaTeX"} (⇧⌥F)`}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => void formatCurrent.current()}
          >
            {formatBusy ? (
              <LoaderCircle size={14} className="spin" />
            ) : (
              <AlignLeft size={14} />
            )}
            Format
          </button>
        </div>
      )}
      <div className="code-editor-content" ref={container} />
    </div>
  );
}
