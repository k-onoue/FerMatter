export interface Card {
  id: string;
  name: string;
  tex: string;
  kind: "inline" | "block" | "figure";
  origin: "source" | "custom";
  file?: string;
  line?: number;
  endLine?: number;
  sourceOffset?: number;
  sourceEnd?: number;
  startUTF16?: number;
  endUTF16?: number;
  sourceOrder: number;
  status: "pending" | "ready" | "failed";
  svgPath?: string;
  pngPath?: string;
  pdfPath?: string;
  error?: string;
  sourceFontSize?: number;
  exportFontSize?: number;
  outlinePath?: string[];
  typstContext?: { file: string; offset: number }[];
  parentID?: string;
  [key: string]: unknown;
}
export interface Heading {
  id: string;
  title: string;
  level: number;
  file: string;
  line: number;
  path: string[];
}
export interface Project {
  id: string;
  root: string;
  main: string;
  format: "latex" | "typst";
  engine: string;
  cards: Card[];
  outline: Heading[];
  customOrder: string[];
  sections: { id: string; name: string }[];
  sectionAssignments: Record<string, string>;
  candidates: string[];
  exportFontSize: number;
  exportFormat: string;
  warnings?: string[];
  [key: string]: unknown;
}
export interface Buffer {
  path: string;
  text: string;
  revision: string;
  bom: boolean;
  newline: string;
  dirty?: boolean;
  recovered?: boolean;
  error?: string;
}
export interface TreeEntry {
  path: string;
  directory: boolean;
}
export type Palette = {
  accent: string;
  windowBackground: string;
  surfaceBackground: string;
  text: string;
  command: string;
  delimiter: string;
  number: string;
  comment: string;
};
export interface Theme {
  schemaVersion: 1;
  kind: string;
  id: string;
  name: string;
  light: Palette;
  dark: Palette;
}
export interface Preferences {
  appearance: "light" | "dark" | "system";
  autoSave: boolean;
  opacity: number;
  theme: string;
  pdfWidth: number;
  [key: string]: unknown;
}
export interface Binding {
  key: string;
  command: boolean;
  control: boolean;
  shift: boolean;
  option: boolean;
}
export interface Keybindings {
  schemaVersion: 1;
  kind: string;
  bindings: Record<string, Binding>;
  extensions?: unknown;
}
export interface Anchor {
  targetID: string;
  page: number;
  x: number;
  y: number;
  width?: number;
  height?: number;
}
export interface DocumentPDF {
  pdfPath: string;
  sourceRoot: string;
  format: string;
  anchors: Anchor[];
  targets: SourceTarget[];
}
export interface SourceTarget {
  id: string;
  parentID?: string;
  name: string;
  kind: string;
  file: string;
  line: number;
  endLine: number;
  startUTF16: number;
  endUTF16: number;
}
