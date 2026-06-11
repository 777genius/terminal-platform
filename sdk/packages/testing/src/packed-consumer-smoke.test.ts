import { describe, expect, it } from "vitest";

import {
  TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC,
  assertTerminalPlatformPackedConsumerSmoke,
  runTerminalPlatformPackedConsumerSmoke,
  type TerminalPlatformPackedConsumerSmokeEntry,
  type TerminalPlatformPackedConsumerSmokeResult,
} from "./index.js";

type Assert<T extends true> = T;
type Equal<Actual, Expected> = (<T>() => T extends Actual ? 1 : 2) extends
  <T>() => T extends Expected ? 1 : 2
  ? true
  : false;

type _SmokeResultFailuresAreReadonly = Assert<
  Equal<
    TerminalPlatformPackedConsumerSmokeResult["failures"],
    readonly {
      readonly specifier: string;
      readonly message: string;
    }[]
  >
>;

describe("Terminal Platform packed consumer smoke helpers", () => {
  it("verifies public package entrypoints through a caller-provided importer", async () => {
    const result = await assertTerminalPlatformPackedConsumerSmoke((specifier) => import(specifier));

    expect(result.ok).toBe(true);
    expect(result.checked).toBe(TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC.length);
    expect(TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC.map((entry) => entry.specifier)).toContain(
      "@terminal-platform/workspace-core/bootstrap",
    );
    expect(TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC.map((entry) => entry.specifier)).toContain(
      "@terminal-platform/testing",
    );
  });

  it("reports missing runtime exports and type-only entrypoint leaks", async () => {
    const entries = [
      {
        specifier: "virtual:runtime",
        expectedRuntimeExports: ["present", "missing"],
      },
      {
        specifier: "virtual:type-only",
        expectNoRuntimeExports: true,
      },
    ] satisfies readonly TerminalPlatformPackedConsumerSmokeEntry[];

    const result = await runTerminalPlatformPackedConsumerSmoke(async (specifier) => {
      if (specifier === "virtual:runtime") {
        return { present: true };
      }

      return { leaked: true };
    }, { entries });

    expect(result).toEqual({
      ok: false,
      checked: 2,
      failures: [
        {
          specifier: "virtual:runtime",
          message: 'missing runtime export "missing"',
        },
        {
          specifier: "virtual:type-only",
          message: "expected no runtime exports, found leaked",
        },
      ],
    });
  });

  it("throws a readable aggregate error for failed assertions", async () => {
    const entries = [{
      specifier: "virtual:missing",
      expectedRuntimeExports: ["ok"],
    }] satisfies readonly TerminalPlatformPackedConsumerSmokeEntry[];

    await expect(assertTerminalPlatformPackedConsumerSmoke(async () => ({}), { entries }))
      .rejects.toThrow([
        "Terminal Platform packed consumer smoke failed:",
        '- virtual:missing: missing runtime export "ok"',
      ].join("\n"));
  });
});
