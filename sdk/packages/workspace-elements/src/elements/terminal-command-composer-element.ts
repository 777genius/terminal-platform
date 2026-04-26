import { html, LitElement } from "lit";

import type {
  TerminalCommandHistoryInputState,
  TerminalCommandHistoryNavigationDirection,
} from "./terminal-command-history-navigation.js";

export type TerminalCommandComposerShortcut = "\u0003" | "\r";

export type TerminalCommandComposerDraftChangeDetail = {
  value: string;
};

export type TerminalCommandComposerHistoryNavigateDetail = {
  direction: TerminalCommandHistoryNavigationDirection;
  input: TerminalCommandHistoryInputState;
};

export type TerminalCommandComposerShortcutDetail = {
  data: TerminalCommandComposerShortcut;
};

export class TerminalCommandComposerElement extends LitElement {
  static override properties = {
    draft: { type: String },
    canWriteInput: { attribute: "can-write-input", type: Boolean },
    canSend: { attribute: "can-send", type: Boolean },
    canPasteClipboard: { attribute: "can-paste-clipboard", type: Boolean },
    placeholder: { type: String },
    pasteTitle: { attribute: "paste-title", type: String },
  };

  declare draft: string;
  declare canWriteInput: boolean;
  declare canSend: boolean;
  declare canPasteClipboard: boolean;
  declare placeholder: string;
  declare pasteTitle: string;

  #pendingFocus = false;

  constructor() {
    super();
    this.draft = "";
    this.canWriteInput = false;
    this.canSend = false;
    this.canPasteClipboard = false;
    this.placeholder = "";
    this.pasteTitle = "Paste clipboard into the focused pane";
  }

  protected override createRenderRoot(): HTMLElement {
    return this;
  }

  override render() {
    return html`
      <span class="prompt" aria-hidden="true">&gt;_</span>
      <textarea
        data-testid="tp-command-input"
        name="tp-command-input"
        .value=${this.draft}
        ?disabled=${!this.canWriteInput}
        placeholder=${this.placeholder}
        aria-label="Focused pane command input"
        @input=${(event: Event) => this.handleInput(event)}
        @keydown=${(event: KeyboardEvent) => this.handleKeydown(event)}
      ></textarea>
      <div
        class="composer-actions"
        part="composer-actions"
        data-testid="tp-command-composer-actions"
        aria-label="Command actions"
      >
        <button
          class="primary"
          type="button"
          data-testid="tp-send-command"
          title="Send command to the focused pane"
          aria-label="Send command to the focused pane"
          ?disabled=${!this.canSend}
          @click=${() => this.dispatchComposerEvent("tp-terminal-command-submit")}
        >
          Run
        </button>
        <button
          type="button"
          data-testid="tp-paste-clipboard"
          title=${this.pasteTitle}
          aria-label="Paste clipboard into the focused pane"
          ?disabled=${!this.canPasteClipboard}
          @click=${() => this.dispatchComposerEvent("tp-terminal-command-paste")}
        >
          Paste
        </button>
        <button
          type="button"
          data-testid="tp-send-interrupt"
          title="Send Ctrl+C to the focused pane"
          aria-label="Send Ctrl+C to the focused pane"
          ?disabled=${!this.canWriteInput}
          @click=${() => this.dispatchShortcut("\u0003")}
        >
          ^C
        </button>
        <button
          type="button"
          data-testid="tp-send-enter"
          title="Send Enter to the focused pane"
          aria-label="Send Enter to the focused pane"
          ?disabled=${!this.canWriteInput}
          @click=${() => this.dispatchShortcut("\r")}
        >
          Enter
        </button>
      </div>
    `;
  }

  protected override updated(): void {
    if (this.#pendingFocus && this.tryFocusInput()) {
      this.#pendingFocus = false;
    }
  }

  focusInput(): boolean {
    if (!this.canWriteInput) {
      this.#pendingFocus = false;
      return false;
    }

    if (this.tryFocusInput()) {
      this.#pendingFocus = false;
      return true;
    }

    this.#pendingFocus = true;
    void this.updateComplete.then(() => {
      if (this.#pendingFocus && this.isConnected && this.tryFocusInput()) {
        this.#pendingFocus = false;
      }
    });
    return true;
  }

  private tryFocusInput(): boolean {
    const textarea = this.commandInput;
    if (!textarea || textarea.disabled) {
      return false;
    }

    textarea.focus({ preventScroll: true });
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    return true;
  }

  applyDraft(value: string): boolean {
    const textarea = this.commandInput;
    if (!textarea) {
      return false;
    }

    textarea.value = value;
    textarea.setSelectionRange(value.length, value.length);
    return true;
  }

  private get commandInput(): HTMLTextAreaElement | null {
    return this.querySelector<HTMLTextAreaElement>('[data-testid="tp-command-input"]');
  }

  private handleInput(event: Event): void {
    const target = event.currentTarget as HTMLTextAreaElement;
    this.dispatchEvent(
      new CustomEvent<TerminalCommandComposerDraftChangeDetail>("tp-terminal-command-draft-change", {
        bubbles: true,
        composed: true,
        detail: {
          value: target.value,
        },
      }),
    );
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) {
      return;
    }

    const target = event.currentTarget as HTMLTextAreaElement;
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      const handled = !this.dispatchEvent(
        new CustomEvent<TerminalCommandComposerHistoryNavigateDetail>("tp-terminal-command-history-navigate", {
          bubbles: true,
          cancelable: true,
          composed: true,
          detail: {
            direction: event.key === "ArrowUp" ? "previous" : "next",
            input: {
              value: target.value,
              selectionStart: target.selectionStart ?? target.value.length,
              selectionEnd: target.selectionEnd ?? target.value.length,
            },
          },
        }),
      );
      if (handled) {
        event.preventDefault();
      }
      return;
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      this.dispatchComposerEvent("tp-terminal-command-submit");
    }
  }

  private dispatchShortcut(data: TerminalCommandComposerShortcut): void {
    this.dispatchEvent(
      new CustomEvent<TerminalCommandComposerShortcutDetail>("tp-terminal-command-shortcut", {
        bubbles: true,
        composed: true,
        detail: { data },
      }),
    );
  }

  private dispatchComposerEvent(type: "tp-terminal-command-paste" | "tp-terminal-command-submit"): void {
    this.dispatchEvent(new CustomEvent(type, { bubbles: true, composed: true }));
  }
}
