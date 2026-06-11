import { describe, expect, it } from "vitest";

import { defineTerminalPlatformElements } from "./define.js";

const terminalPlatformElementTags = [
  "tp-terminal-workspace",
  "tp-terminal-session-list",
  "tp-terminal-status-bar",
  "tp-terminal-command-composer",
  "tp-terminal-command-dock",
  "tp-terminal-toolbar",
  "tp-terminal-tab-strip",
  "tp-terminal-screen",
  "tp-terminal-pane-tree",
  "tp-terminal-saved-sessions",
] as const;

describe("workspace elements define public subpath", () => {
  it("defines terminal platform elements once against a host-provided registry", () => {
    const registry = createCustomElementRegistryRecorder();

    defineTerminalPlatformElements(registry);
    defineTerminalPlatformElements(registry);

    expect(registry.definedTags).toEqual(terminalPlatformElementTags);
    for (const tagName of terminalPlatformElementTags) {
      expect(registry.get(tagName)).toBeTypeOf("function");
    }
  });

  it("does nothing when no registry is available", () => {
    const originalRegistry = globalThis.customElements;

    try {
      Reflect.deleteProperty(globalThis, "customElements");
      expect(() => defineTerminalPlatformElements()).not.toThrow();
    } finally {
      if (originalRegistry) {
        Object.defineProperty(globalThis, "customElements", {
          configurable: true,
          value: originalRegistry,
        });
      }
    }
  });
});

interface CustomElementRegistryRecorder extends CustomElementRegistry {
  readonly definedTags: readonly string[];
}

function createCustomElementRegistryRecorder(): CustomElementRegistryRecorder {
  const constructors = new Map<string, CustomElementConstructor>();
  const definedTags: string[] = [];

  return {
    get(tagName) {
      return constructors.get(tagName);
    },
    define(tagName, constructor) {
      definedTags.push(tagName);
      constructors.set(tagName, constructor);
    },
    getName() {
      return undefined;
    },
    upgrade() {},
    whenDefined(tagName) {
      const constructor = constructors.get(tagName);
      return constructor
        ? Promise.resolve(constructor)
        : Promise.reject(new Error(`custom element "${tagName}" is not defined`));
    },
    definedTags,
  };
}
