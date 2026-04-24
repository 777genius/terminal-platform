export interface TerminalKeyboardInputEvent {
  readonly key: string;
  readonly altKey: boolean;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly isComposing?: boolean;
}

export function terminalInputForKeyboardEvent(event: TerminalKeyboardInputEvent): string | null {
  if (event.isComposing || event.altKey || event.metaKey) {
    return null;
  }

  if (event.ctrlKey) {
    return controlInputForKey(event.key);
  }

  switch (event.key) {
    case "Enter":
      return "\r";
    case "Tab":
      return "\t";
    case "Backspace":
      return "\u007f";
    case "Escape":
      return "\u001b";
    case "ArrowUp":
      return "\u001b[A";
    case "ArrowDown":
      return "\u001b[B";
    case "ArrowRight":
      return "\u001b[C";
    case "ArrowLeft":
      return "\u001b[D";
    case "Home":
      return "\u001b[H";
    case "End":
      return "\u001b[F";
    case "Delete":
      return "\u001b[3~";
    case "PageUp":
      return "\u001b[5~";
    case "PageDown":
      return "\u001b[6~";
    default:
      return event.key.length === 1 ? event.key : null;
  }
}

function controlInputForKey(key: string): string | null {
  switch (key.toLowerCase()) {
    case "a":
      return "\u0001";
    case "c":
      return "\u0003";
    case "d":
      return "\u0004";
    case "e":
      return "\u0005";
    case "k":
      return "\u000b";
    case "l":
      return "\u000c";
    case "u":
      return "\u0015";
    case "w":
      return "\u0017";
    default:
      return null;
  }
}
