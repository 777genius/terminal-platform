export interface TerminalScreenShortcutEvent {
  readonly key: string;
  readonly altKey: boolean;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey?: boolean;
  readonly isComposing?: boolean;
}

export function isTerminalScreenSearchShortcut(event: TerminalScreenShortcutEvent): boolean {
  if (event.isComposing || event.altKey || event.shiftKey) {
    return false;
  }

  return (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f";
}
