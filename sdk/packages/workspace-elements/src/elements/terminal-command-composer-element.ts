import { html, LitElement, nothing, type PropertyValues } from "lit";

import {
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
  resolveTerminalCommandComposerActionPlacement,
  resolveTerminalCommandComposerActions,
  type TerminalCommandComposerActionId,
  type TerminalCommandComposerActionLabelOverride,
  type TerminalCommandComposerActionPlacement,
  type TerminalCommandComposerActionPresentation,
  type TerminalCommandComposerShortcut,
} from "./terminal-command-composer-actions.js";
import {
  resolveTerminalCommandComposerAutocomplete,
  type TerminalCommandComposerAutocompletePresentation,
} from "./terminal-command-composer-autocomplete.js";
import {
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  type TerminalCommandComposerAutocompleteAcceptDetail,
  type TerminalCommandComposerAutocompleteDismissDetail,
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
  TerminalCommandComposerAutocompleteAcceptDetail,
  TerminalCommandComposerAutocompleteDismissDetail,
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
    canInterrupt: { attribute: "can-interrupt", type: Boolean },
    canPasteClipboard: { attribute: "can-paste-clipboard", type: Boolean },
    autocompleteSuggestion: {
      attribute: "autocomplete-suggestion",
      type: String,
    },
    maxRows: { attribute: "max-rows", type: Number },
    minRows: { attribute: "min-rows", type: Number },
    inputDescriptionId: { attribute: "input-description-id", type: String },
    actionLabels: { attribute: false },
    actionsLabel: { attribute: "actions-label", type: String },
    placeholder: { type: String },
    pasteTitle: { attribute: "paste-title", type: String },
    placement: { type: String },
  };

  declare draft: string;
  declare canWriteInput: boolean;
  declare canSend: boolean;
  declare canInterrupt: boolean;
  declare canPasteClipboard: boolean;
  declare autocompleteSuggestion: string | null;
  declare maxRows: number;
  declare minRows: number;
  declare inputDescriptionId: string;
  declare actionLabels:
    | Partial<
        Record<
          TerminalCommandComposerActionId,
          TerminalCommandComposerActionLabelOverride
        >
      >
    | null
    | undefined;
  declare actionsLabel: string;
  declare placeholder: string;
  declare pasteTitle: string;
  declare placement: TerminalCommandComposerActionPlacement;

  #pendingFocus = false;
  #inputFocused = false;
  #inputSelectionStart = 0;
  #inputSelectionEnd = 0;
  #isComposing = false;

  constructor() {
    super();
    this.draft = "";
    this.canWriteInput = false;
    this.canSend = false;
    this.canInterrupt = false;
    this.canPasteClipboard = false;
    this.autocompleteSuggestion = null;
    this.maxRows = TERMINAL_COMMAND_COMPOSER_DEFAULT_MAX_ROWS;
    this.minRows = TERMINAL_COMMAND_COMPOSER_DEFAULT_MIN_ROWS;
    this.inputDescriptionId = "";
    this.actionLabels = null;
    this.actionsLabel = "Command actions";
    this.placeholder = "";
    this.pasteTitle = TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE;
    this.placement = "panel";
  }

  protected override createRenderRoot(): HTMLElement {
    return this;
  }

  override render() {
    const rowCount = this.resolveRowCount(this.draft);
    const placement = resolveTerminalCommandComposerActionPlacement(
      this.placement,
    );
    const actions = resolveTerminalCommandComposerActions({
      actionLabels: this.actionLabels ?? null,
      pasteTitle: this.pasteTitle,
      placement,
      terminalActions: {
        canInterrupt: this.canInterrupt,
        canSend: this.canSend,
      },
    });
    const actionsTemplate =
      actions.length > 0
        ? html`
            <div
              class="composer-actions"
              part="composer-actions"
              data-action-placement=${placement}
              data-testid="tp-command-composer-actions"
              aria-label=${this.actionsLabel || "Command actions"}
            >
              ${actions.map((action) => this.renderAction(action))}
            </div>
          `
        : nothing;
    const autocomplete = this.resolveAutocompletePresentation({
      value: this.draft,
      selectionStart: this.#inputSelectionStart,
      selectionEnd: this.#inputSelectionEnd,
    });
    // prettier-ignore
    const autocompleteTemplate = autocomplete
      ? html`<div class="autocomplete-ghost" part="autocomplete-ghost" data-testid="tp-command-autocomplete-ghost" data-prefix-length=${autocomplete.draft.length} style=${`--tp-command-autocomplete-prefix-width: ${autocomplete.draft.length}ch;`} aria-hidden="true"><span class="autocomplete-ghost-prefix">${autocomplete.draft}</span><span class="autocomplete-ghost-suffix">${autocomplete.suffix}</span></div>`
      : nothing;

    return html`
      <span class="prompt" part="prompt" aria-hidden="true">&gt;_</span>
      <div
        class="command-input-stack"
        part="input-stack"
        data-autocomplete-visible=${String(Boolean(autocomplete))}
        data-testid="tp-command-input-stack"
      >
        ${autocompleteTemplate}
        <textarea
          data-multiline=${String(rowCount > 1)}
          data-row-count=${String(rowCount)}
          data-testid="tp-command-input"
          part="input"
          name="tp-command-input"
          .value=${this.draft}
          ?disabled=${!this.canWriteInput}
          autocomplete="off"
          autocapitalize="off"
          autocorrect="off"
          enterkeyhint="send"
          placeholder=${this.placeholder}
          spellcheck="false"
          aria-label="Focused pane command input"
          aria-describedby=${this.inputDescriptionId || nothing}
          rows=${rowCount}
          @blur=${() => this.setInputFocused(false)}
          @click=${(event: MouseEvent) =>
            this.refreshInputSelection(
              event.currentTarget as HTMLTextAreaElement,
            )}
          @compositionend=${(event: CompositionEvent) =>
            this.handleCompositionEnd(event)}
          @compositionstart=${() => this.setIsComposing(true)}
          @focus=${(event: FocusEvent) => this.handleInputFocus(event)}
          @input=${(event: Event) => this.handleInput(event)}
          @keydown=${(event: KeyboardEvent) => this.handleKeydown(event)}
          @keyup=${(event: KeyboardEvent) =>
            this.refreshInputSelection(
              event.currentTarget as HTMLTextAreaElement,
            )}
          @select=${(event: Event) =>
            this.refreshInputSelection(
              event.currentTarget as HTMLTextAreaElement,
            )}
        ></textarea>
      </div>
      ${actionsTemplate}
    `;
  }

  protected override updated(changedProperties: PropertyValues): void {
    if (
      changedProperties.has("draft") ||
      changedProperties.has("maxRows") ||
      changedProperties.has("minRows")
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
    this.setInputFocused(true);
    this.refreshInputSelection(textarea);
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
    this.refreshInputSelection(textarea);
    return true;
  }

  private get commandInput(): HTMLTextAreaElement | null {
    return this.querySelector<HTMLTextAreaElement>(
      '[data-testid="tp-command-input"]',
    );
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
    this.refreshInputSelection(target);
    this.dispatchDraftChange(target.value);
  }

  private handleInputFocus(event: FocusEvent): void {
    this.setInputFocused(true);
    this.refreshInputSelection(event.currentTarget as HTMLTextAreaElement);
  }

  private handleCompositionEnd(event: CompositionEvent): void {
    this.setIsComposing(false);
    this.refreshInputSelection(event.currentTarget as HTMLTextAreaElement);
  }

  private setInputFocused(inputFocused: boolean): void {
    if (this.#inputFocused === inputFocused) {
      return;
    }

    this.#inputFocused = inputFocused;
    this.requestUpdate();
  }

  private setIsComposing(isComposing: boolean): void {
    if (this.#isComposing === isComposing) {
      return;
    }

    this.#isComposing = isComposing;
    this.requestUpdate();
  }

  private refreshInputSelection(textarea: HTMLTextAreaElement): void {
    const selectionStart = textarea.selectionStart ?? textarea.value.length;
    const selectionEnd = textarea.selectionEnd ?? selectionStart;
    if (
      this.#inputSelectionStart === selectionStart &&
      this.#inputSelectionEnd === selectionEnd
    ) {
      return;
    }

    this.#inputSelectionStart = selectionStart;
    this.#inputSelectionEnd = selectionEnd;
    this.requestUpdate();
  }

  private insertTextAtSelection(
    textarea: HTMLTextAreaElement,
    text: string,
  ): void {
    const selectionStart = textarea.selectionStart ?? textarea.value.length;
    const selectionEnd = textarea.selectionEnd ?? selectionStart;
    const nextValue = `${textarea.value.slice(0, selectionStart)}${text}${textarea.value.slice(selectionEnd)}`;
    const nextSelection = selectionStart + text.length;

    textarea.value = nextValue;
    textarea.setSelectionRange(nextSelection, nextSelection);
    this.syncCommandInputHeight(textarea);
    this.dispatchDraftChange(nextValue);
  }

  private dispatchDraftChange(value: string): void {
    this.dispatchEvent(
      new CustomEvent<TerminalCommandComposerDraftChangeDetail>(
        TERMINAL_COMMAND_COMPOSER_EVENTS.draftChange,
        {
          bubbles: true,
          composed: true,
          detail: {
            value,
          },
        },
      ),
    );
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (
      event.defaultPrevented ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey
    ) {
      return;
    }

    const target = event.currentTarget as HTMLTextAreaElement;
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      const handled = !this.dispatchEvent(
        new CustomEvent<TerminalCommandComposerHistoryNavigateDetail>(
          TERMINAL_COMMAND_COMPOSER_EVENTS.historyNavigate,
          {
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
          },
        ),
      );
      if (handled) {
        event.preventDefault();
      }
      return;
    }

    if (event.key === "Tab" && event.shiftKey) {
      event.preventDefault();
      this.insertTextAtSelection(target, "\n");
      return;
    }

    if (event.key === "Tab") {
      const autocomplete = this.resolveAutocompletePresentation({
        value: target.value,
        selectionStart: target.selectionStart ?? target.value.length,
        selectionEnd: target.selectionEnd ?? target.value.length,
      });
      if (autocomplete) {
        event.preventDefault();
        this.dispatchAutocompleteAccept(autocomplete);
      }
      return;
    }

    if (event.key === "Escape") {
      const autocomplete = this.resolveAutocompletePresentation({
        value: target.value,
        selectionStart: target.selectionStart ?? target.value.length,
        selectionEnd: target.selectionEnd ?? target.value.length,
      });
      if (autocomplete) {
        event.preventDefault();
        this.dispatchAutocompleteDismiss(autocomplete);
      }
      return;
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      this.dispatchComposerEvent(TERMINAL_COMMAND_COMPOSER_EVENTS.submit);
    }
  }

  private resolveAutocompletePresentation(input: {
    value: string;
    selectionStart: number;
    selectionEnd: number;
  }): TerminalCommandComposerAutocompletePresentation | null {
    return resolveTerminalCommandComposerAutocomplete(input, {
      canWriteInput: this.canWriteInput,
      inputFocused: this.#inputFocused,
      isComposing: this.#isComposing,
      suggestion: this.autocompleteSuggestion,
    });
  }

  private dispatchAutocompleteAccept(
    autocomplete: TerminalCommandComposerAutocompletePresentation,
  ): void {
    this.dispatchEvent(
      new CustomEvent<TerminalCommandComposerAutocompleteAcceptDetail>(
        TERMINAL_COMMAND_COMPOSER_EVENTS.autocompleteAccept,
        {
          bubbles: true,
          composed: true,
          detail: {
            draft: autocomplete.draft,
            suggestion: autocomplete.suggestion,
            value: autocomplete.suggestion,
          },
        },
      ),
    );
  }

  private dispatchAutocompleteDismiss(
    autocomplete: TerminalCommandComposerAutocompletePresentation,
  ): void {
    this.dispatchEvent(
      new CustomEvent<TerminalCommandComposerAutocompleteDismissDetail>(
        TERMINAL_COMMAND_COMPOSER_EVENTS.autocompleteDismiss,
        {
          bubbles: true,
          composed: true,
          detail: {
            draft: autocomplete.draft,
            suggestion: autocomplete.suggestion,
          },
        },
      ),
    );
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
        title=${action.placement === "terminal" ? nothing : action.title}
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
        return !this.canWriteInput;
      case TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt:
        return !this.canWriteInput || !this.canInterrupt;
    }
  }

  private handleActionClick(
    action: TerminalCommandComposerActionPresentation,
  ): void {
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
      new CustomEvent<TerminalCommandComposerShortcutDetail>(
        TERMINAL_COMMAND_COMPOSER_EVENTS.shortcut,
        {
          bubbles: true,
          composed: true,
          detail: { data },
        },
      ),
    );
  }

  private dispatchComposerEvent(
    type:
      | typeof TERMINAL_COMMAND_COMPOSER_EVENTS.paste
      | typeof TERMINAL_COMMAND_COMPOSER_EVENTS.submit,
  ): void {
    this.dispatchEvent(
      new CustomEvent(type, { bubbles: true, composed: true }),
    );
  }
}
