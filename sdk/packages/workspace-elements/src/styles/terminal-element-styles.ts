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
