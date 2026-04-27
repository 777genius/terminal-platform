import { css, unsafeCSS } from "lit";

import { terminalPlatformDefaultThemeCssText } from "@terminal-platform/design-tokens";

export const terminalElementStyles = css`
  ${unsafeCSS(terminalPlatformDefaultThemeCssText)}

  :host {
    display: block;
    color: var(--tp-color-text);
    font-family: var(--tp-font-family-ui);
    line-height: 1.4;
  }

  :host([hidden]) {
    display: none;
  }

  *,
  *::before,
  *::after {
    box-sizing: border-box;
  }

  .panel {
    background: var(--tp-color-panel);
    border: 1px solid var(--tp-color-border);
    border-radius: var(--tp-radius-md);
    box-shadow: var(--tp-shadow-panel);
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
    background: color-mix(in srgb, var(--tp-color-panel-raised) 82%, transparent);
    color: inherit;
    border-radius: var(--tp-radius-sm);
    padding: 0.45rem 0.7rem;
    cursor: pointer;
    font: inherit;
  }

  button:hover {
    border-color: var(--tp-color-accent);
  }

  button:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--tp-color-accent) 62%, transparent);
    outline-offset: 2px;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  button[data-danger="true"] {
    border-color: color-mix(in srgb, var(--tp-color-danger) 42%, transparent);
    color: var(--tp-color-danger);
  }

  button[data-action-tone="primary"],
  button[data-quick-command-tone="primary"],
  button[data-screen-action-tone="primary"] {
    border-color: color-mix(in srgb, var(--tp-color-accent) 52%, transparent);
    background: color-mix(in srgb, var(--tp-color-accent) 16%, var(--tp-color-panel-raised));
  }

  button[data-screen-action-tone="success"] {
    border-color: color-mix(in srgb, var(--tp-color-success) 52%, transparent);
    color: var(--tp-color-success);
  }

  button[data-screen-action-tone="danger"],
  button[data-session-action-tone="danger"] {
    border-color: color-mix(in srgb, var(--tp-color-danger) 46%, transparent);
    color: var(--tp-color-danger);
  }

  button[data-confirming="true"] {
    background: color-mix(in srgb, var(--tp-color-danger) 16%, var(--tp-color-panel-raised));
  }

  code,
  pre {
    font-family: var(--tp-font-family-mono);
  }
`;
