import type { TerminalCreateNativeSessionInput } from "../../contracts/terminal-workspace-contracts.js";

export function buildCreateNativeSessionPayload(input: {
  title: string;
  program: string;
  args: string;
  cwd: string;
}): TerminalCreateNativeSessionInput {
  const program = input.program.trim();
  const title = input.title.trim();
  const cwd = input.cwd.trim();

  return {
    ...(title ? { title } : {}),
    ...(program
      ? {
          launch: {
            program,
            args: parseLaunchArgs(input.args),
            ...(cwd ? { cwd } : {}),
          },
        }
      : {}),
  };
}

export function parseLaunchArgs(value: string): string[] {
  const matches = value.match(/(?:[^\s"]+|"[^"]*")+/g);
  if (!matches) {
    return [];
  }

  return matches.map((entry) => entry.replace(/^"|"$/g, ""));
}
