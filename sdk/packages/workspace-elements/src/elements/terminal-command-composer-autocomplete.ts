export const TERMINAL_COMMAND_COMPOSER_AUTOCOMPLETE_DEFAULT_MIN_LENGTH = 2;
export const TERMINAL_COMMAND_COMPOSER_AUTOCOMPLETE_DEFAULT_MAX_LENGTH = 160;

export interface TerminalCommandComposerAutocompleteInputState {
  readonly value: string;
  readonly selectionStart: number;
  readonly selectionEnd: number;
}

export interface TerminalCommandComposerAutocompleteOptions {
  readonly canWriteInput: boolean;
  readonly inputFocused: boolean;
  readonly isComposing?: boolean;
  readonly maxLength?: number;
  readonly minLength?: number;
  readonly suggestion?: string | null;
}

export interface TerminalCommandComposerAutocompletePresentation {
  readonly draft: string;
  readonly suffix: string;
  readonly suggestion: string;
}

export function resolveTerminalCommandComposerAutocomplete(
  input: TerminalCommandComposerAutocompleteInputState,
  options: TerminalCommandComposerAutocompleteOptions,
): TerminalCommandComposerAutocompletePresentation | null {
  const minLength =
    normalizePositiveInteger(options.minLength) ??
    TERMINAL_COMMAND_COMPOSER_AUTOCOMPLETE_DEFAULT_MIN_LENGTH;
  const maxLength =
    normalizePositiveInteger(options.maxLength) ??
    TERMINAL_COMMAND_COMPOSER_AUTOCOMPLETE_DEFAULT_MAX_LENGTH;
  const suggestion = options.suggestion ?? "";

  if (
    !options.canWriteInput ||
    !options.inputFocused ||
    options.isComposing ||
    suggestion.length === 0 ||
    input.value.length < minLength ||
    input.value.length > maxLength ||
    hasLineBreak(input.value) ||
    hasLineBreak(suggestion) ||
    input.selectionStart !== input.value.length ||
    input.selectionEnd !== input.value.length ||
    !suggestion.startsWith(input.value) ||
    suggestion.length <= input.value.length
  ) {
    return null;
  }

  return {
    draft: input.value,
    suffix: suggestion.slice(input.value.length),
    suggestion,
  };
}

function hasLineBreak(value: string): boolean {
  return value.includes("\n") || value.includes("\r");
}

function normalizePositiveInteger(
  value: number | null | undefined,
): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }

  const normalized = Math.trunc(value);
  return normalized > 0 ? normalized : null;
}
