import {
  DEFAULT_TERMINAL_FONT_SCALE,
  terminalPlatformTerminalFontScales,
  type TerminalPlatformTerminalFontScale,
} from "../read-models/workspace-snapshot.js";
import type { ServiceContext } from "./service-context.js";

export class TerminalDisplayPreferenceService {
  readonly #context: ServiceContext;

  constructor(context: ServiceContext) {
    this.#context = context;
  }

  setFontScale(fontScale: string): void {
    const normalizedFontScale = normalizeTerminalFontScale(fontScale);
    if (!normalizedFontScale) {
      this.#context.recordDiagnostic({
        code: "terminal_display_preference_unsupported",
        message: `Terminal font scale "${fontScale}" is not supported`,
        severity: "warn",
        recoverable: true,
      });
      return;
    }

    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      terminalDisplay: {
        ...snapshot.terminalDisplay,
        fontScale: normalizedFontScale,
      },
    }));
  }

  setLineWrap(lineWrap: boolean): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      terminalDisplay: {
        ...snapshot.terminalDisplay,
        lineWrap,
      },
    }));
  }
}

export function normalizeTerminalFontScale(
  fontScale: string | null | undefined,
): TerminalPlatformTerminalFontScale | null {
  const normalizedFontScale = fontScale?.trim();
  if (!normalizedFontScale) {
    return DEFAULT_TERMINAL_FONT_SCALE;
  }

  return isTerminalFontScale(normalizedFontScale) ? normalizedFontScale : null;
}

export function isTerminalFontScale(
  value: string,
): value is TerminalPlatformTerminalFontScale {
  return (terminalPlatformTerminalFontScales as readonly string[]).includes(value);
}
