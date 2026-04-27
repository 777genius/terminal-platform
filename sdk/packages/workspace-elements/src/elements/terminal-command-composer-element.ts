import { html, LitElement, nothing, type PropertyValues } from "lit";

import {
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
  resolveTerminalCommandComposerActionPlacement,
  resolveTerminalCommandComposerActions,
  type TerminalCommandComposerActionId,
  type TerminalCommandComposerActionPlacement,
  type TerminalCommandComposerActionPresentation,
  type TerminalCommandComposerShortcut,
} from "./terminal-command-composer-actions.js";
import {
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  type TerminalCommandComposerDraftChangeDetail,
  type TerminalCommandComposerHistoryNavigateDetail,
  type TerminalCommandComposerShortcutDetail,
} from "./terminal-command-composer-events.js";
import {
  TERMINAL_COMMAND_COMPOSER_DEFAULT_MAX_ROWS,
  TERMINAL_COMMAND_COMPOSER_DEFAULT_MIN_ROWS,
  resolveTerminalCommandComposerRows,
} from "./terminal-command-composer-layout.js";

export type { TerminalCommandComposerShortcut } from "./terminal-command-composer-actions.js";
export { TERMINAL_COMMAND_COMPOSER_EVENTS } from "./terminal-command-composer-events.js";
export type {
  TerminalCommandComposerDraftChangeDetail,
  TerminalCommandComposerEventMap,
  TerminalCommandComposerEventType,
  TerminalCommandComposerHistoryNavigateDetail,
  TerminalCommandComposerShortcutDetail,
} from "./terminal-command-composer-events.js";

export class TerminalCommandComposerElement extends LitElement {
  static override properties = {
    draft: { type: String },
    canWriteInput: { attribute: "can-write-input", type: Boolean },
    canSend: { attribute: "can-send", type: Boolean },
    canPasteClipboard: { attribute: "can-paste-clipboard", type: Boolean },
    maxRows: { attribute: "max-rows", type: Number },
    minRows: { attribute: "min-rows", type: Number },
    placeholder: { type: String },
    pasteTitle: { attribute: "paste-title", type: String },
    placement: { type: String },
  };

  declare draft: string;
  declare canWriteInput: boolean;
  declare canSend: boolean;
  declare canPasteClipboard: boolean;
  declare maxRows: number;
  declare minRows: number;
  declare placeholder: string;
  declare pasteTitle: string;
  declare placement: TerminalCommandComposerActionPlacement;

  #pendingFocus = false;

  constructor() {
    super();
    this.draft = "";
    this.canWriteInput = false;
    this.canSend = false;
    this.canPasteClipboard = false;
    this.maxRows = TERMINAL_COMMAND_COMPOSER_DEFAULT_MAX_ROWS;
    this.minRows = TERMINAL_COMMAND_COMPOSER_DEFAULT_MIN_ROWS;
    this.placeholder = "";
    this.pasteTitle = TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE;
    this.placement = "panel";
  }

  protected override createRenderRoot(): HTMLElement {
    return this;
  }

  override render() {
    const rowCount = this.resolveRowCount(this.draft);
    const placement = resolveTerminalCommandComposerActionPlacement(this.placement);
    const actions = resolveTerminalCommandComposerActions({
      pasteTitle: this.pasteTitle,
      placement,
    });

    return html`
      <span class="prompt" part="prompt" aria-hidden="true">&gt;_</span>
      <textarea
        data-multiline=${String(rowCount > 1)}
        data-row-count=${String(rowCount)}
        data-testid="tp-command-input"
        part="input"
        name="tp-command-input"
        .value=${this.draft}
        ?disabled=${!this.canWriteInput}
        placeholder=${this.placeholder}
        aria-label="Focused pane command input"
        rows=${rowCount}
        @input=${(event: Event) => this.handleInput(event)}
        @keydown=${(event: KeyboardEvent) => this.handleKeydown(event)}
      ></textarea>
      <div
        class="composer-actions"
        part="composer-actions"
        data-action-placement=${placement}
        data-testid="tp-command-composer-actions"
        aria-label="Command actions"
      >
        ${actions.map((action) => this.renderAction(action))}
      </div>
    `;
  }

  protected override updated(changedProperties: PropertyValues): void {
    if (
      changedProperties.has("draft")
      || changedProperties.has("maxRows")
      || changedProperties.has("minRows")
    ) {
      this.syncCommandInputHeight();
    }

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
    this.syncCommandInputHeight(textarea);
    textarea.setSelectionRange(value.length, value.length);
    return true;
  }

  private get commandInput(): HTMLTextAreaElement | null {
    return this.querySelector<HTMLTextAreaElement>('[data-testid="tp-command-input"]');
  }

  private resolveRowCount(value: string): number {
    return resolveTerminalCommandComposerRows(value, {
      maxRows: this.maxRows,
      minRows: this.minRows,
    });
  }

  private syncCommandInputHeight(textarea = this.commandInput): void {
    if (!textarea) {
      return;
    }

    const rowCount = this.resolveRowCount(textarea.value);
    textarea.rows = rowCount;
    textarea.dataset.rowCount = String(rowCount);
    textarea.dataset.multiline = String(rowCount > 1);
    textarea.style.height = "auto";
    if (textarea.scrollHeight > 0) {
      textarea.style.height = `${textarea.scrollHeight}px`;
    }
  }

  private handleInput(event: Event): void {
    const target = event.currentTarget as HTMLTextAreaElement;
    this.syncCommandInputHeight(target);
    this.dispatchEvent(
      new CustomEvent<TerminalCommandComposerDraftChangeDetail>(TERMINAL_COMMAND_COMPOSER_EVENTS.draftChange, {
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
        new CustomEvent<TerminalCommandComposerHistoryNavigateDetail>(TERMINAL_COMMAND_COMPOSER_EVENTS.historyNavigate, {
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
      this.dispatchComposerEvent(TERMINAL_COMMAND_COMPOSER_EVENTS.submit);
    }
  }

  private renderAction(action: TerminalCommandComposerActionPresentation) {
    const disabled = this.isActionDisabled(action.id);

    return html`
      <button
        class=${action.primary ? "primary" : ""}
        part=${action.part}
        type="button"
        data-action=${action.id}
        data-action-disabled=${String(disabled)}
        data-action-label-mode=${action.labelMode}
        data-action-placement=${action.placement}
        data-action-tone=${action.tone}
        data-key-hint=${action.keyHint ?? nothing}
        data-testid=${action.testId}
        title=${action.title}
        aria-label=${action.ariaLabel}
        aria-keyshortcuts=${action.ariaKeyShortcuts ?? nothing}
        ?disabled=${disabled}
        @click=${() => this.handleActionClick(action)}
      >
        ${action.label}
      </button>
    `;
  }

  private isActionDisabled(actionId: TerminalCommandComposerActionId): boolean {
    switch (actionId) {
      case TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit:
        return !this.canSend;
      case TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste:
        return !this.canPasteClipboard;
      case TERMINAL_COMMAND_COMPOSER_ACTION_IDS.enter:
      case TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt:
        return !this.canWriteInput;
    }
  }

  private handleActionClick(action: TerminalCommandComposerActionPresentation): void {
    if (action.shortcut) {
      this.dispatchShortcut(action.shortcut);
      return;
    }

    if (action.id === TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste) {
      this.dispatchComposerEvent(TERMINAL_COMMAND_COMPOSER_EVENTS.paste);
      return;
    }

    this.dispatchComposerEvent(TERMINAL_COMMAND_COMPOSER_EVENTS.submit);
  }

  private dispatchShortcut(data: TerminalCommandComposerShortcut): void {
    this.dispatchEvent(
      new CustomEvent<TerminalCommandComposerShortcutDetail>(TERMINAL_COMMAND_COMPOSER_EVENTS.shortcut, {
        bubbles: true,
        composed: true,
        detail: { data },
      }),
    );
  }

  private dispatchComposerEvent(
    type: typeof TERMINAL_COMMAND_COMPOSER_EVENTS.paste | typeof TERMINAL_COMMAND_COMPOSER_EVENTS.submit,
  ): void {
    this.dispatchEvent(new CustomEvent(type, { bubbles: true, composed: true }));
  }
}
