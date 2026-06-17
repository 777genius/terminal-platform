import type {
  ScreenColor,
  ScreenSurfacePalette,
  ScreenTextStyle,
  ScreenUnderlineStyle,
} from "@terminal-platform/runtime-types";

export type TerminalOutputStyleMap = Record<string, string | undefined>;

export const PLAIN_SCREEN_TEXT_STYLE: ScreenTextStyle = {
  foreground: null,
  background: null,
  underline_color: null,
  bold: false,
  dim: false,
  italic: false,
  blink: false,
  underline: null,
  overline: false,
  border: null,
  inverse: false,
  hidden: false,
  strikethrough: false,
  hyperlink: null,
};

const TERMINAL_NAMED_COLOR_CSS: Record<string, string> = {
  black: "#111827",
  red: "#ef4444",
  green: "#22c55e",
  yellow: "#eab308",
  blue: "#3b82f6",
  magenta: "#a855f7",
  cyan: "#06b6d4",
  white: "#e5e7eb",
  bright_black: "#6b7280",
  bright_red: "#f87171",
  bright_green: "#4ade80",
  bright_yellow: "#facc15",
  bright_blue: "#60a5fa",
  bright_magenta: "#c084fc",
  bright_cyan: "#22d3ee",
  bright_white: "#f9fafb",
  dim_black: "#030712",
  dim_red: "#991b1b",
  dim_green: "#166534",
  dim_yellow: "#854d0e",
  dim_blue: "#1d4ed8",
  dim_magenta: "#7e22ce",
  dim_cyan: "#0e7490",
  dim_white: "#9ca3af",
  foreground: "var(--tp-terminal-color-text)",
  bright_foreground: "var(--tp-terminal-color-text)",
  dim_foreground: "var(--tp-terminal-color-text-muted)",
  background: "var(--tp-terminal-color-bg)",
  cursor: "var(--tp-terminal-color-accent)",
};

const TERMINAL_X11_NAMED_COLOR_CSS: Record<string, string> = {
  aliceblue: "rgb(240 248 255)",
  antiquewhite: "rgb(250 235 215)",
  aqua: "rgb(0 255 255)",
  aquamarine: "rgb(127 255 212)",
  azure: "rgb(240 255 255)",
  beige: "rgb(245 245 220)",
  bisque: "rgb(255 228 196)",
  blanchedalmond: "rgb(255 235 205)",
  blueviolet: "rgb(138 43 226)",
  brown: "rgb(165 42 42)",
  burlywood: "rgb(222 184 135)",
  cadetblue: "rgb(95 158 160)",
  chartreuse: "rgb(127 255 0)",
  chocolate: "rgb(210 105 30)",
  coral: "rgb(255 127 80)",
  cornflowerblue: "rgb(100 149 237)",
  cornsilk: "rgb(255 248 220)",
  crimson: "rgb(220 20 60)",
  darkblue: "rgb(0 0 139)",
  darkcyan: "rgb(0 139 139)",
  darkgoldenrod: "rgb(184 134 11)",
  darkgray: "rgb(169 169 169)",
  darkgrey: "rgb(169 169 169)",
  darkgreen: "rgb(0 100 0)",
  darkkhaki: "rgb(189 183 107)",
  darkmagenta: "rgb(139 0 139)",
  darkolivegreen: "rgb(85 107 47)",
  darkorange: "rgb(255 140 0)",
  darkorchid: "rgb(153 50 204)",
  darkred: "rgb(139 0 0)",
  darksalmon: "rgb(233 150 122)",
  darkseagreen: "rgb(143 188 143)",
  darkslateblue: "rgb(72 61 139)",
  darkslategray: "rgb(47 79 79)",
  darkslategrey: "rgb(47 79 79)",
  darkturquoise: "rgb(0 206 209)",
  darkviolet: "rgb(148 0 211)",
  deeppink: "rgb(255 20 147)",
  deepskyblue: "rgb(0 191 255)",
  dimgray: "rgb(105 105 105)",
  dimgrey: "rgb(105 105 105)",
  dodgerblue: "rgb(30 144 255)",
  firebrick: "rgb(178 34 34)",
  floralwhite: "rgb(255 250 240)",
  forestgreen: "rgb(34 139 34)",
  fuchsia: "rgb(255 0 255)",
  gainsboro: "rgb(220 220 220)",
  ghostwhite: "rgb(248 248 255)",
  gold: "rgb(255 215 0)",
  goldenrod: "rgb(218 165 32)",
  gray: "rgb(128 128 128)",
  greenyellow: "rgb(173 255 47)",
  grey: "rgb(128 128 128)",
  honeydew: "rgb(240 255 240)",
  hotpink: "rgb(255 105 180)",
  indianred: "rgb(205 92 92)",
  indigo: "rgb(75 0 130)",
  ivory: "rgb(255 255 240)",
  khaki: "rgb(240 230 140)",
  lavender: "rgb(230 230 250)",
  lavenderblush: "rgb(255 240 245)",
  lawngreen: "rgb(124 252 0)",
  lemonchiffon: "rgb(255 250 205)",
  lightblue: "rgb(173 216 230)",
  lightcoral: "rgb(240 128 128)",
  lightcyan: "rgb(224 255 255)",
  lightgoldenrod: "rgb(238 221 130)",
  lightgoldenrodyellow: "rgb(250 250 210)",
  lightgray: "rgb(211 211 211)",
  lightgreen: "rgb(144 238 144)",
  lightgrey: "rgb(211 211 211)",
  lightpink: "rgb(255 182 193)",
  lightsalmon: "rgb(255 160 122)",
  lightseagreen: "rgb(32 178 170)",
  lightskyblue: "rgb(135 206 250)",
  lightslategray: "rgb(119 136 153)",
  lightslategrey: "rgb(119 136 153)",
  lightsteelblue: "rgb(176 196 222)",
  lightyellow: "rgb(255 255 224)",
  lime: "rgb(0 255 0)",
  limegreen: "rgb(50 205 50)",
  linen: "rgb(250 240 230)",
  maroon: "rgb(176 48 96)",
  mediumaquamarine: "rgb(102 205 170)",
  mediumblue: "rgb(0 0 205)",
  mediumorchid: "rgb(186 85 211)",
  mediumpurple: "rgb(147 112 219)",
  mediumseagreen: "rgb(60 179 113)",
  mediumslateblue: "rgb(123 104 238)",
  mediumspringgreen: "rgb(0 250 154)",
  mediumturquoise: "rgb(72 209 204)",
  mediumvioletred: "rgb(199 21 133)",
  midnightblue: "rgb(25 25 112)",
  mintcream: "rgb(245 255 250)",
  mistyrose: "rgb(255 228 225)",
  moccasin: "rgb(255 228 181)",
  navajowhite: "rgb(255 222 173)",
  navy: "rgb(0 0 128)",
  oldlace: "rgb(253 245 230)",
  olive: "rgb(128 128 0)",
  olivedrab: "rgb(107 142 35)",
  orange: "rgb(255 165 0)",
  orangered: "rgb(255 69 0)",
  orchid: "rgb(218 112 214)",
  palegoldenrod: "rgb(238 232 170)",
  palegreen: "rgb(152 251 152)",
  paleturquoise: "rgb(175 238 238)",
  palevioletred: "rgb(219 112 147)",
  papayawhip: "rgb(255 239 213)",
  peachpuff: "rgb(255 218 185)",
  peru: "rgb(205 133 63)",
  pink: "rgb(255 192 203)",
  plum: "rgb(221 160 221)",
  powderblue: "rgb(176 224 230)",
  purple: "rgb(160 32 240)",
  rebeccapurple: "rgb(102 51 153)",
  rosybrown: "rgb(188 143 143)",
  royalblue: "rgb(65 105 225)",
  saddlebrown: "rgb(139 69 19)",
  salmon: "rgb(250 128 114)",
  sandybrown: "rgb(244 164 96)",
  seagreen: "rgb(46 139 87)",
  seashell: "rgb(255 245 238)",
  sienna: "rgb(160 82 45)",
  silver: "rgb(192 192 192)",
  skyblue: "rgb(135 206 235)",
  slateblue: "rgb(106 90 205)",
  slategray: "rgb(112 128 144)",
  slategrey: "rgb(112 128 144)",
  snow: "rgb(255 250 250)",
  springgreen: "rgb(0 255 127)",
  steelblue: "rgb(70 130 180)",
  tan: "rgb(210 180 140)",
  teal: "rgb(0 128 128)",
  thistle: "rgb(216 191 216)",
  tomato: "rgb(255 99 71)",
  turquoise: "rgb(64 224 208)",
  violet: "rgb(238 130 238)",
  wheat: "rgb(245 222 179)",
  whitesmoke: "rgb(245 245 245)",
  yellowgreen: "rgb(154 205 50)",
};

const TERMINAL_ANSI_16_COLOR_CSS = [
  TERMINAL_NAMED_COLOR_CSS.black,
  TERMINAL_NAMED_COLOR_CSS.red,
  TERMINAL_NAMED_COLOR_CSS.green,
  TERMINAL_NAMED_COLOR_CSS.yellow,
  TERMINAL_NAMED_COLOR_CSS.blue,
  TERMINAL_NAMED_COLOR_CSS.magenta,
  TERMINAL_NAMED_COLOR_CSS.cyan,
  TERMINAL_NAMED_COLOR_CSS.white,
  TERMINAL_NAMED_COLOR_CSS.bright_black,
  TERMINAL_NAMED_COLOR_CSS.bright_red,
  TERMINAL_NAMED_COLOR_CSS.bright_green,
  TERMINAL_NAMED_COLOR_CSS.bright_yellow,
  TERMINAL_NAMED_COLOR_CSS.bright_blue,
  TERMINAL_NAMED_COLOR_CSS.bright_magenta,
  TERMINAL_NAMED_COLOR_CSS.bright_cyan,
  TERMINAL_NAMED_COLOR_CSS.bright_white,
];

const TERMINAL_256_COLOR_STEPS = [0, 95, 135, 175, 215, 255] as const;

export function terminalColorToCss(
  color: ScreenColor | null | undefined,
): string | undefined {
  if (!color) {
    return undefined;
  }

  if (color.kind === "named") {
    return terminalNamedColorToCss(color.name);
  }

  if (color.kind === "rgb") {
    const red = clampTerminalColorChannel(color.r);
    const green = clampTerminalColorChannel(color.g);
    const blue = clampTerminalColorChannel(color.b);
    return red == null || green == null || blue == null
      ? undefined
      : `rgb(${red} ${green} ${blue})`;
  }

  return terminalIndexedColorToCss(color.index);
}

export function resolveTerminalOutputStyle(
  style: ScreenTextStyle,
): TerminalOutputStyleMap {
  let foreground = terminalColorToCss(style.foreground);
  let background = terminalColorToCss(style.background);
  if (style.inverse) {
    const nextForeground =
      background ??
      "var(--tp-terminal-surface-background-color, var(--tp-terminal-color-bg))";
    const nextBackground =
      foreground ??
      "var(--tp-terminal-surface-foreground-color, var(--tp-terminal-color-text))";
    foreground = nextForeground;
    background = nextBackground;
  }

  const decorationLines = [
    style.underline ? "underline" : "",
    style.overline ? "overline" : "",
    style.strikethrough ? "line-through" : "",
  ].filter(Boolean);

  return {
    animation: style.blink
      ? "terminal-output-blink 1s steps(1, end) infinite"
      : undefined,
    backgroundColor: background,
    borderRadius:
      style.border === "encircled"
        ? "999px"
        : style.border === "framed"
          ? "0.12rem"
          : undefined,
    color: style.hidden ? "transparent" : foreground,
    fontStyle: style.italic ? "italic" : undefined,
    fontWeight: style.bold ? (style.dim ? "760" : "700") : undefined,
    opacity: style.dim ? "0.72" : undefined,
    outline: style.border ? "1px solid currentColor" : undefined,
    outlineOffset: style.border === "encircled" ? "-1px" : undefined,
    fontSize: style.baseline ? "0.78em" : undefined,
    textDecorationColor: terminalColorToCss(style.underline_color),
    textDecorationLine:
      decorationLines.length > 0 ? decorationLines.join(" ") : undefined,
    textDecorationStyle: terminalUnderlineStyleToCss(style.underline),
    textShadow: style.hidden ? "none" : undefined,
    verticalAlign:
      style.baseline === "superscript"
        ? "super"
        : style.baseline === "subscript"
          ? "sub"
          : undefined,
  };
}

export function resolveTerminalSurfacePaletteStyle(
  palette: ScreenSurfacePalette | null | undefined,
): TerminalOutputStyleMap {
  const foreground = terminalColorToCss(palette?.foreground);
  const background = terminalColorToCss(palette?.background);
  const cursor = terminalColorToCss(palette?.cursor);
  return {
    background,
    caretColor: cursor,
    color: foreground,
    "--tp-terminal-surface-background-color": background,
    "--tp-terminal-surface-cursor-color": cursor,
    "--tp-terminal-surface-foreground-color": foreground,
  };
}

export function normalizeTerminalHyperlink(
  value: string | null | undefined,
): string | null {
  if (!value) {
    return null;
  }

  const trimmed = value.trim();
  if (!trimmed || /[\u0000-\u001f\u007f]/u.test(trimmed)) {
    return null;
  }

  try {
    const url = new URL(trimmed);
    return ["https:", "http:", "mailto:", "file:", "ftp:"].includes(url.protocol)
      ? url.href
      : null;
  } catch {
    return null;
  }
}

function clampTerminalColorChannel(value: number): number | null {
  return Number.isFinite(value)
    ? Math.max(0, Math.min(255, Math.trunc(value)))
    : null;
}

function normalizeTerminalNamedColorName(name: string): string {
  return name
    .trim()
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase()
    .replace(/[\s-]+/gu, "_");
}

function normalizeTerminalCompactColorName(name: string): string {
  return name
    .trim()
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase()
    .replace(/[\s_-]+/gu, "");
}

function terminalNamedColorToCss(name: string): string | undefined {
  const normalized = normalizeTerminalNamedColorName(name);
  const compact = normalizeTerminalCompactColorName(name);
  const mapped =
    TERMINAL_NAMED_COLOR_CSS[normalized] ??
    TERMINAL_X11_NAMED_COLOR_CSS[normalized] ??
    TERMINAL_NAMED_COLOR_CSS[compact] ??
    TERMINAL_X11_NAMED_COLOR_CSS[compact];
  if (mapped) {
    return mapped;
  }

  const grayMatch = normalized.match(/^gr[ae]y_?(\d{1,3})$/u);
  if (!grayMatch) {
    return undefined;
  }
  const percentage = Number(grayMatch[1]);
  if (!Number.isInteger(percentage) || percentage < 0 || percentage > 100) {
    return undefined;
  }
  const channel = Math.floor((percentage * 255 + 50) / 100);
  return `rgb(${channel} ${channel} ${channel})`;
}

function terminalIndexedColorToCss(index: number): string | undefined {
  if (!Number.isFinite(index)) {
    return undefined;
  }
  const normalized = Math.trunc(index);
  if (normalized < 0 || normalized > 255) {
    return undefined;
  }
  if (normalized < TERMINAL_ANSI_16_COLOR_CSS.length) {
    return TERMINAL_ANSI_16_COLOR_CSS[normalized];
  }
  if (normalized <= 231) {
    const offset = normalized - 16;
    const red = TERMINAL_256_COLOR_STEPS[Math.floor(offset / 36)] ?? 0;
    const green = TERMINAL_256_COLOR_STEPS[Math.floor((offset % 36) / 6)] ?? 0;
    const blue = TERMINAL_256_COLOR_STEPS[offset % 6] ?? 0;
    return `rgb(${red} ${green} ${blue})`;
  }
  const gray = 8 + (normalized - 232) * 10;
  return `rgb(${gray} ${gray} ${gray})`;
}

function terminalUnderlineStyleToCss(
  style: ScreenUnderlineStyle | null | undefined,
): string | undefined {
  if (!style || style === "single") {
    return undefined;
  }
  return style === "curly" ? "wavy" : style;
}
