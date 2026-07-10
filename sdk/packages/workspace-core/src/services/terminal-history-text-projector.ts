import type {
  ScreenColor,
  ScreenLine,
  ScreenLineSpan,
  ScreenTextStyle,
} from "@terminal-platform/runtime-types";

const DEFAULT_COLUMNS = 80;
const DEFAULT_ROWS = 24;
const TAB_WIDTH = 8;

interface TerminalHistoryCell {
  readonly style: ScreenTextStyle;
  readonly text: string;
}

interface TerminalHistoryRow {
  cells: Array<TerminalHistoryCell | undefined>;
  wrapped: boolean;
}

export interface TerminalHistoryTextProjection {
  readonly endsWithLineBreak: boolean;
  readonly lines: ScreenLine[];
  readonly startsWithLineBreak: boolean;
}

export interface TerminalHistoryTextProjectorOptions {
  readonly columns?: number;
  readonly rows?: number;
}

/**
 * Replays durable raw VT output into rendered history lines.
 *
 * Pane history is a terminal byte stream, not plain text. In particular, shells
 * redraw editable input with backspace, carriage return, cursor movement, and
 * erase controls. Removing those controls would preserve superseded cells and
 * corrupt commands such as `p\bprint` into `pprint`.
 */
export function projectTerminalHistoryText(
  text: string,
  options: TerminalHistoryTextProjectorOptions = {}
): TerminalHistoryTextProjection {
  const projector = new TerminalHistoryTextProjector(options);
  projector.write(text);
  return {
    lines: projector.lines(),
    startsWithLineBreak: startsWithTerminalLineBreak(text),
    endsWithLineBreak: endsWithTerminalLineBreak(text),
  };
}

class TerminalHistoryTextProjector {
  readonly #columns: number;
  readonly #rows: number;
  readonly #screen: TerminalHistoryRow[] = [];
  #cursorColumn = 0;
  #cursorRow = 0;
  #currentStyle = defaultScreenTextStyle();
  #savedCursor: { column: number; row: number } | null = null;
  #wrapPending = false;

  constructor(options: TerminalHistoryTextProjectorOptions) {
    this.#columns = positiveInteger(options.columns, DEFAULT_COLUMNS);
    this.#rows = positiveInteger(options.rows, DEFAULT_ROWS);
    this.#ensureRow(0);
  }

  write(text: string): void {
    let index = 0;
    while (index < text.length) {
      const code = text.charCodeAt(index);

      if (code === 0x1b) {
        index = this.#consumeEscapeSequence(text, index);
        continue;
      }
      if (code < 0x20 || code === 0x7f) {
        this.#applyControl(code);
        index += 1;
        continue;
      }

      const codePoint = text.codePointAt(index);
      if (codePoint === undefined) {
        break;
      }
      const character = String.fromCodePoint(codePoint);
      this.#writeCharacter(character);
      index += character.length;
    }
  }

  lines(): ScreenLine[] {
    const lines = this.#screen.map((row) => screenLineFromRow(row));
    while (lines.length > 0 && lines.at(-1)?.text === "" && !lines.at(-1)?.wrapped) {
      lines.pop();
    }
    return lines;
  }

  #consumeEscapeSequence(text: string, escapeIndex: number): number {
    const introducer = text[escapeIndex + 1];
    if (!introducer) {
      return text.length;
    }

    if (introducer === "[") {
      let finalIndex = escapeIndex + 2;
      while (finalIndex < text.length) {
        const code = text.charCodeAt(finalIndex);
        if (code >= 0x40 && code <= 0x7e) {
          this.#applyCsi(text.slice(escapeIndex + 2, finalIndex), text[finalIndex] ?? "");
          return finalIndex + 1;
        }
        finalIndex += 1;
      }
      return text.length;
    }

    if (introducer === "]") {
      return consumeStringControl(text, escapeIndex + 2, true);
    }
    if (introducer === "P" || introducer === "_" || introducer === "^") {
      return consumeStringControl(text, escapeIndex + 2, false);
    }

    if (introducer === "7") {
      this.#saveCursor();
    } else if (introducer === "8") {
      this.#restoreCursor();
    } else if (introducer === "D") {
      this.#lineFeed();
    } else if (introducer === "E") {
      this.#carriageReturn();
      this.#lineFeed();
    } else if (introducer === "M") {
      this.#cursorRow = Math.max(0, this.#cursorRow - 1);
      this.#ensureRow(this.#cursorRow);
      this.#wrapPending = false;
    } else if (introducer === "c") {
      this.#reset();
    }

    return escapeIndex + 2;
  }

  #applyControl(code: number): void {
    if (code === 0x08) {
      this.#backspace();
      return;
    }
    if (code === 0x09) {
      const nextTabStop = Math.min(
        this.#columns - 1,
        (Math.floor(this.#cursorColumn / TAB_WIDTH) + 1) * TAB_WIDTH
      );
      while (this.#cursorColumn < nextTabStop) {
        this.#writeCharacter(" ");
      }
      return;
    }
    if (code === 0x0a || code === 0x0b || code === 0x0c) {
      this.#lineFeed();
      return;
    }
    if (code === 0x0d) {
      this.#carriageReturn();
    }
  }

  #applyCsi(parametersText: string, final: string): void {
    const parameters = parseCsiParameters(parametersText);
    const first = parameters[0] ?? 0;
    const count = Math.max(1, first || 1);

    if (final === "A") {
      this.#cursorRow = Math.max(0, this.#cursorRow - count);
    } else if (final === "B") {
      this.#cursorRow += count;
      this.#ensureRow(this.#cursorRow);
    } else if (final === "C" || final === "a") {
      this.#cursorColumn = Math.min(this.#columns - 1, this.#cursorColumn + count);
    } else if (final === "D") {
      this.#cursorColumn = Math.max(0, this.#cursorColumn - count);
    } else if (final === "E") {
      this.#cursorRow += count;
      this.#cursorColumn = 0;
      this.#ensureRow(this.#cursorRow);
    } else if (final === "F") {
      this.#cursorRow = Math.max(0, this.#cursorRow - count);
      this.#cursorColumn = 0;
    } else if (final === "G" || final === "`") {
      this.#cursorColumn = clampColumn((first || 1) - 1, this.#columns);
    } else if (final === "H" || final === "f") {
      const viewportTop = Math.max(0, this.#screen.length - this.#rows);
      this.#cursorRow = viewportTop + Math.max(0, (parameters[0] || 1) - 1);
      this.#cursorColumn = clampColumn((parameters[1] || 1) - 1, this.#columns);
      this.#ensureRow(this.#cursorRow);
    } else if (final === "J") {
      this.#eraseDisplay(first);
    } else if (final === "K") {
      this.#eraseLine(first);
    } else if (final === "P") {
      this.#deleteCharacters(count);
    } else if (final === "@") {
      this.#insertBlankCharacters(count);
    } else if (final === "X") {
      this.#eraseCharacters(count);
    } else if (final === "m") {
      this.#applySgr(parameters);
    } else if (final === "s") {
      this.#saveCursor();
    } else if (final === "u") {
      this.#restoreCursor();
    }

    if (final !== "m") {
      this.#wrapPending = false;
    }
  }

  #writeCharacter(character: string): void {
    if (this.#wrapPending) {
      this.#ensureRow(this.#cursorRow).wrapped = true;
      this.#cursorRow += 1;
      this.#cursorColumn = 0;
      this.#ensureRow(this.#cursorRow);
      this.#wrapPending = false;
    }

    const row = this.#ensureRow(this.#cursorRow);
    row.cells[this.#cursorColumn] = {
      text: character,
      style: this.#currentStyle,
    };

    if (this.#cursorColumn >= this.#columns - 1) {
      this.#wrapPending = true;
    } else {
      this.#cursorColumn += 1;
    }
  }

  #backspace(): void {
    if (this.#wrapPending) {
      this.#wrapPending = false;
      return;
    }
    this.#cursorColumn = Math.max(0, this.#cursorColumn - 1);
  }

  #carriageReturn(): void {
    this.#cursorColumn = 0;
    this.#wrapPending = false;
  }

  #lineFeed(): void {
    this.#cursorRow += 1;
    this.#ensureRow(this.#cursorRow);
    this.#wrapPending = false;
  }

  #eraseLine(mode: number): void {
    const row = this.#ensureRow(this.#cursorRow);
    if (mode === 1) {
      row.cells.splice(0, this.#cursorColumn + 1, ...new Array(this.#cursorColumn + 1));
    } else if (mode === 2) {
      row.cells = [];
      row.wrapped = false;
    } else {
      row.cells.length = Math.min(row.cells.length, this.#cursorColumn);
    }
  }

  #eraseDisplay(mode: number): void {
    if (mode === 1) {
      for (let rowIndex = 0; rowIndex < this.#cursorRow; rowIndex += 1) {
        const row = this.#ensureRow(rowIndex);
        row.cells = [];
        row.wrapped = false;
      }
      this.#eraseLine(1);
      return;
    }

    if (mode === 2 || mode === 3) {
      const viewportTop = Math.max(0, this.#cursorRow - this.#rows + 1);
      const viewportBottom = viewportTop + this.#rows;
      for (let rowIndex = viewportTop; rowIndex < viewportBottom; rowIndex += 1) {
        const row = this.#ensureRow(rowIndex);
        row.cells = [];
        row.wrapped = false;
      }
      if (mode === 3 && viewportTop > 0) {
        this.#screen.splice(0, viewportTop);
        this.#cursorRow -= viewportTop;
      }
      return;
    }

    this.#eraseLine(0);
    this.#screen.splice(this.#cursorRow + 1);
  }

  #deleteCharacters(count: number): void {
    const row = this.#ensureRow(this.#cursorRow);
    row.cells.splice(this.#cursorColumn, count);
  }

  #insertBlankCharacters(count: number): void {
    const row = this.#ensureRow(this.#cursorRow);
    row.cells.splice(this.#cursorColumn, 0, ...new Array(count));
    row.cells.length = Math.min(row.cells.length, this.#columns);
  }

  #eraseCharacters(count: number): void {
    const row = this.#ensureRow(this.#cursorRow);
    for (
      let column = this.#cursorColumn;
      column < Math.min(this.#columns, this.#cursorColumn + count);
      column += 1
    ) {
      row.cells[column] = undefined;
    }
  }

  #applySgr(parameters: readonly number[]): void {
    const values = parameters.length > 0 ? [...parameters] : [0];
    let style = cloneScreenTextStyle(this.#currentStyle);
    for (let index = 0; index < values.length; index += 1) {
      const value = values[index] ?? 0;
      if (value === 0) {
        style = defaultScreenTextStyle();
      } else if (value === 1) {
        style.bold = true;
      } else if (value === 2) {
        style.dim = true;
      } else if (value === 3) {
        style.italic = true;
      } else if (value === 4) {
        style.underline = "single";
      } else if (value === 5 || value === 6) {
        style.blink = true;
      } else if (value === 7) {
        style.inverse = true;
      } else if (value === 8) {
        style.hidden = true;
      } else if (value === 9) {
        style.strikethrough = true;
      } else if (value === 22) {
        style.bold = false;
        style.dim = false;
      } else if (value === 23) {
        style.italic = false;
      } else if (value === 24) {
        style.underline = null;
      } else if (value === 25) {
        style.blink = false;
      } else if (value === 27) {
        style.inverse = false;
      } else if (value === 28) {
        style.hidden = false;
      } else if (value === 29) {
        style.strikethrough = false;
      } else if (value === 39) {
        style.foreground = null;
      } else if (value === 49) {
        style.background = null;
      } else if (value >= 30 && value <= 37) {
        style.foreground = namedAnsiColor(value - 30, false);
      } else if (value >= 40 && value <= 47) {
        style.background = namedAnsiColor(value - 40, false);
      } else if (value >= 90 && value <= 97) {
        style.foreground = namedAnsiColor(value - 90, true);
      } else if (value >= 100 && value <= 107) {
        style.background = namedAnsiColor(value - 100, true);
      } else if (value === 38 || value === 48 || value === 58) {
        const extended = parseExtendedColor(values, index + 1);
        if (extended) {
          if (value === 38) style.foreground = extended.color;
          if (value === 48) style.background = extended.color;
          if (value === 58) style.underline_color = extended.color;
          index = extended.lastIndex;
        }
      } else if (value === 59) {
        style.underline_color = null;
      }
    }
    this.#currentStyle = style;
  }

  #saveCursor(): void {
    this.#savedCursor = { column: this.#cursorColumn, row: this.#cursorRow };
  }

  #restoreCursor(): void {
    if (!this.#savedCursor) return;
    this.#cursorColumn = clampColumn(this.#savedCursor.column, this.#columns);
    this.#cursorRow = Math.max(0, this.#savedCursor.row);
    this.#ensureRow(this.#cursorRow);
    this.#wrapPending = false;
  }

  #reset(): void {
    this.#screen.length = 0;
    this.#cursorColumn = 0;
    this.#cursorRow = 0;
    this.#currentStyle = defaultScreenTextStyle();
    this.#savedCursor = null;
    this.#wrapPending = false;
    this.#ensureRow(0);
  }

  #ensureRow(index: number): TerminalHistoryRow {
    while (this.#screen.length <= index) {
      this.#screen.push({ cells: [], wrapped: false });
    }
    return this.#screen[index]!;
  }
}

function screenLineFromRow(row: TerminalHistoryRow): ScreenLine {
  let lastCellIndex = row.cells.length - 1;
  while (lastCellIndex >= 0 && isTrimmableTrailingCell(row.cells[lastCellIndex])) {
    lastCellIndex -= 1;
  }

  const cells = row.cells.slice(0, lastCellIndex + 1);
  const spans: ScreenLineSpan[] = [];
  for (const cell of cells) {
    const normalizedCell = cell ?? {
      text: " ",
      style: defaultScreenTextStyle(),
    };
    const previous = spans.at(-1);
    if (previous && screenTextStylesEqual(previous.style, normalizedCell.style)) {
      previous.text += normalizedCell.text;
    } else {
      spans.push({
        text: normalizedCell.text,
        style: cloneScreenTextStyle(normalizedCell.style),
      });
    }
  }

  const text = spans.map((span) => span.text).join("");
  return {
    text,
    spans,
    ...(row.wrapped ? { wrapped: true } : {}),
  };
}

function isTrimmableTrailingCell(cell: TerminalHistoryCell | undefined): boolean {
  return (
    !cell || (cell.text === " " && screenTextStylesEqual(cell.style, defaultScreenTextStyle()))
  );
}

function consumeStringControl(text: string, startIndex: number, allowBell: boolean): number {
  let index = startIndex;
  while (index < text.length) {
    if (allowBell && text.charCodeAt(index) === 0x07) {
      return index + 1;
    }
    if (text.charCodeAt(index) === 0x1b && text[index + 1] === "\\") {
      return index + 2;
    }
    index += 1;
  }
  return text.length;
}

function parseCsiParameters(text: string): number[] {
  const normalized = text.replace(/^[?!>]/u, "").replace(/[:]/gu, ";");
  if (!normalized) return [];
  return normalized.split(";").map((value) => {
    const parsed = Number.parseInt(value, 10);
    return Number.isFinite(parsed) ? parsed : 0;
  });
}

function parseExtendedColor(
  values: readonly number[],
  startIndex: number
): { color: ScreenColor; lastIndex: number } | null {
  const mode = values[startIndex];
  if (mode === 5 && Number.isInteger(values[startIndex + 1])) {
    return {
      color: { kind: "indexed", index: clampByte(values[startIndex + 1] ?? 0) },
      lastIndex: startIndex + 1,
    };
  }
  if (
    mode === 2 &&
    Number.isInteger(values[startIndex + 1]) &&
    Number.isInteger(values[startIndex + 2]) &&
    Number.isInteger(values[startIndex + 3])
  ) {
    return {
      color: {
        kind: "rgb",
        r: clampByte(values[startIndex + 1] ?? 0),
        g: clampByte(values[startIndex + 2] ?? 0),
        b: clampByte(values[startIndex + 3] ?? 0),
      },
      lastIndex: startIndex + 3,
    };
  }
  return null;
}

function namedAnsiColor(index: number, bright: boolean): ScreenColor {
  const names = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
  const name = names[index] ?? "white";
  return { kind: "named", name: bright ? `bright_${name}` : name };
}

function defaultScreenTextStyle(): ScreenTextStyle {
  return {
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
}

function cloneScreenTextStyle(style: ScreenTextStyle): ScreenTextStyle {
  return {
    ...style,
    foreground: style.foreground ? { ...style.foreground } : null,
    background: style.background ? { ...style.background } : null,
    underline_color: style.underline_color ? { ...style.underline_color } : null,
  };
}

function screenTextStylesEqual(left: ScreenTextStyle, right: ScreenTextStyle): boolean {
  return (
    screenColorsEqual(left.foreground, right.foreground) &&
    screenColorsEqual(left.background, right.background) &&
    screenColorsEqual(left.underline_color, right.underline_color) &&
    left.bold === right.bold &&
    left.dim === right.dim &&
    left.italic === right.italic &&
    left.blink === right.blink &&
    left.underline === right.underline &&
    left.overline === right.overline &&
    left.border === right.border &&
    left.baseline === right.baseline &&
    left.inverse === right.inverse &&
    left.hidden === right.hidden &&
    left.strikethrough === right.strikethrough &&
    left.hyperlink === right.hyperlink
  );
}

function screenColorsEqual(left: ScreenColor | null, right: ScreenColor | null): boolean {
  if (left === right) return true;
  if (!left || !right || left.kind !== right.kind) return false;
  if (left.kind === "named" && right.kind === "named") return left.name === right.name;
  if (left.kind === "indexed" && right.kind === "indexed") return left.index === right.index;
  return (
    left.kind === "rgb" &&
    right.kind === "rgb" &&
    left.r === right.r &&
    left.g === right.g &&
    left.b === right.b
  );
}

function startsWithTerminalLineBreak(text: string): boolean {
  return text.startsWith("\n") || text.startsWith("\r");
}

function endsWithTerminalLineBreak(text: string): boolean {
  return text.endsWith("\n") || text.endsWith("\r");
}

function positiveInteger(value: number | undefined, fallback: number): number {
  return typeof value === "number" && Number.isInteger(value) && value > 0 ? value : fallback;
}

function clampColumn(value: number, columns: number): number {
  return Math.max(0, Math.min(columns - 1, value));
}

function clampByte(value: number): number {
  return Math.max(0, Math.min(255, value));
}
