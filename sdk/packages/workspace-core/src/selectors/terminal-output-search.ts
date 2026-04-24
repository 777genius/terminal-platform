export interface TerminalOutputSearchOptions {
  activeMatchIndex?: number | null;
}

export interface TerminalOutputSearchTextSegment {
  kind: "text";
  value: string;
}

export interface TerminalOutputSearchMatchSegment {
  kind: "match";
  value: string;
  matchIndex: number;
  active: boolean;
}

export type TerminalOutputSearchSegment =
  | TerminalOutputSearchTextSegment
  | TerminalOutputSearchMatchSegment;

export interface TerminalOutputSearchLine {
  lineIndex: number;
  text: string;
  matchCount: number;
  segments: TerminalOutputSearchSegment[];
}

export interface TerminalOutputSearchResult {
  query: string;
  matchCount: number;
  activeMatchIndex: number | null;
  lines: TerminalOutputSearchLine[];
}

export function createTerminalOutputSearchResult(
  lines: readonly string[],
  query: string,
  options: TerminalOutputSearchOptions = {},
): TerminalOutputSearchResult {
  const normalizedQuery = normalizeTerminalOutputSearchQuery(query);
  const matchCount = countTerminalOutputSearchMatches(lines, normalizedQuery);
  const activeMatchIndex = resolveTerminalOutputSearchMatchIndex(
    options.activeMatchIndex ?? null,
    matchCount,
  );
  let matchOffset = 0;

  return {
    query: normalizedQuery,
    matchCount,
    activeMatchIndex,
    lines: lines.map((line, lineIndex) => {
      const searchLine = createTerminalOutputSearchLine(
        line,
        lineIndex,
        normalizedQuery,
        activeMatchIndex,
        matchOffset,
      );
      matchOffset += searchLine.matchCount;
      return searchLine;
    }),
  };
}

export function normalizeTerminalOutputSearchQuery(query: string): string {
  return query.trim();
}

export function countTerminalOutputSearchMatches(
  lines: readonly string[],
  query: string,
): number {
  const normalizedQuery = normalizeTerminalOutputSearchQuery(query);
  if (!normalizedQuery) {
    return 0;
  }

  return lines.reduce((count, line) => {
    return count + countTerminalOutputSearchMatchesInLine(line, normalizedQuery);
  }, 0);
}

export function countTerminalOutputSearchMatchesInLine(
  line: string,
  query: string,
): number {
  const normalizedQuery = normalizeTerminalOutputSearchQuery(query);
  if (!normalizedQuery) {
    return 0;
  }

  const lowerLine = line.toLocaleLowerCase();
  const lowerQuery = normalizedQuery.toLocaleLowerCase();
  let count = 0;
  let cursor = 0;
  let matchIndex = lowerLine.indexOf(lowerQuery);
  while (matchIndex >= 0) {
    count += 1;
    cursor = matchIndex + normalizedQuery.length;
    matchIndex = lowerLine.indexOf(lowerQuery, cursor);
  }

  return count;
}

export function resolveTerminalOutputSearchMatchIndex(
  activeMatchIndex: number | null,
  matchCount: number,
): number | null {
  if (matchCount === 0) {
    return null;
  }

  if (activeMatchIndex === null) {
    return 0;
  }

  return Math.max(0, Math.min(activeMatchIndex, matchCount - 1));
}

export function formatTerminalOutputSearchCount(
  query: string,
  matchCount: number,
  activeMatchIndex: number | null,
): string {
  if (!normalizeTerminalOutputSearchQuery(query)) {
    return "Search output";
  }

  if (matchCount === 0 || activeMatchIndex === null) {
    return "0 matches";
  }

  return `${activeMatchIndex + 1} of ${matchCount}`;
}

export function serializeTerminalOutputLines(lines: readonly string[]): string {
  return lines.join("\n").replace(/\s+$/u, "");
}

function createTerminalOutputSearchLine(
  text: string,
  lineIndex: number,
  query: string,
  activeMatchIndex: number | null,
  matchOffset: number,
): TerminalOutputSearchLine {
  if (!query || text.length === 0) {
    return {
      lineIndex,
      text,
      matchCount: 0,
      segments: [{ kind: "text", value: text.length > 0 ? text : " " }],
    };
  }

  const lowerText = text.toLocaleLowerCase();
  const lowerQuery = query.toLocaleLowerCase();
  const segments: TerminalOutputSearchSegment[] = [];
  let cursor = 0;
  let localMatchIndex = 0;
  let matchIndex = lowerText.indexOf(lowerQuery);

  while (matchIndex >= 0) {
    if (matchIndex > cursor) {
      segments.push({ kind: "text", value: text.slice(cursor, matchIndex) });
    }

    const matchEnd = matchIndex + query.length;
    const globalMatchIndex = matchOffset + localMatchIndex;
    segments.push({
      kind: "match",
      value: text.slice(matchIndex, matchEnd),
      matchIndex: globalMatchIndex,
      active: activeMatchIndex === globalMatchIndex,
    });
    cursor = matchEnd;
    localMatchIndex += 1;
    matchIndex = lowerText.indexOf(lowerQuery, cursor);
  }

  if (cursor < text.length) {
    segments.push({ kind: "text", value: text.slice(cursor) });
  }

  return {
    lineIndex,
    text,
    matchCount: localMatchIndex,
    segments,
  };
}
