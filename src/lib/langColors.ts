// One color per language, shared by the hub cards and the stats tab.
// Names cover both our backend's extension map and tokei's display names.

export const LANG_COLORS: Record<string, string> = {
  Rust: "#b7410e",
  TypeScript: "#3178c6",
  TSX: "#3178c6",
  JavaScript: "#f1c40f",
  JSX: "#f1c40f",
  Svelte: "#ff3e00",
  Vue: "#42b883",
  Python: "#3572a5",
  Ruby: "#cc342d",
  Go: "#00add8",
  Java: "#b07219",
  Kotlin: "#a97bff",
  Swift: "#f05138",
  C: "#555555",
  "C++": "#f34b7d",
  "C Header": "#555555",
  "C++ Header": "#f34b7d",
  "C#": "#178600",
  PHP: "#4f5d95",
  Elixir: "#6e4a7e",
  Zig: "#ec915c",
  Lua: "#000080",
  CSS: "#563d7c",
  Sass: "#563d7c",
  HTML: "#e34c26",
  Shell: "#89e051",
  SQL: "#336791",
  Astro: "#ff5d01",
  Jupyter: "#f37726",
  "Jupyter Notebooks": "#f37726",
  Docker: "#2496ed",
  Dockerfile: "#2496ed",
  Markdown: "#8f877b",
  JSON: "#a8a29a",
  TOML: "#a8a29a",
  YAML: "#a8a29a",
  "Plain Text": "#a8a29a",
};

/// Known color, or a stable hue derived from the name so unknown
/// languages still look intentional.
export function langColor(name: string): string {
  const known = LANG_COLORS[name];
  if (known) return known;
  const hue = [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);
  return `hsl(${hue}, 45%, 45%)`;
}
