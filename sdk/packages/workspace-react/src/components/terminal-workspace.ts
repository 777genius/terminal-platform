import * as React from "react";
import { createComponent } from "@lit/react";

import {
  TerminalWorkspaceElement,
  defineTerminalPlatformElements,
} from "@terminal-platform/workspace-elements";

defineTerminalPlatformElements();

export const TerminalWorkspace = createComponent({
  react: React,
  tagName: "tp-terminal-workspace",
  elementClass: TerminalWorkspaceElement,
  displayName: "TerminalWorkspace",
});
