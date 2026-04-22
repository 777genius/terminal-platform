import { css, unsafeCSS } from "lit";

import { terminalPlatformDefaultThemeCssText } from "@terminal-platform/design-tokens";

export const terminalElementStyles = css`
  ${unsafeCSS(terminalPlatformDefaultThemeCssText)}

  :host {
    display: block;
    color: var(--tp-color-text);
    font-family: var(--tp-font-family-mono);
  }

  :host([hidden]) {
    display: none;
  }

  .panel {
    background: var(--tp-color-panel);
    border: 1px solid var(--tp-color-border);
    border-radius: var(--tp-radius-md);
  }

  .muted {
    color: var(--tp-color-text-muted);
  }

  .panel-header {
    display: grid;
    gap: 0.2rem;
    margin-bottom: var(--tp-space-3);
  }

  .panel-eyebrow {
    color: var(--tp-color-accent);
    font-size: 0.72rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .panel-title {
    font-size: 1rem;
    font-weight: 600;
  }

  .panel-copy {
    color: var(--tp-color-text-muted);
    font-size: 0.88rem;
    line-height: 1.45;
  }

  .empty-state {
    border: 1px dashed var(--tp-color-border);
    border-radius: var(--tp-radius-md);
    padding: var(--tp-space-3);
    color: var(--tp-color-text-muted);
  }

  button {
    appearance: none;
    border: 1px solid var(--tp-color-border);
    background: transparent;
    color: inherit;
    border-radius: var(--tp-radius-sm);
    padding: 0.35rem 0.6rem;
    cursor: pointer;
    font: inherit;
  }

  button:hover {
    border-color: var(--tp-color-accent);
  }
`;
