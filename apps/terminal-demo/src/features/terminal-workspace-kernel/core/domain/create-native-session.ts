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
  const args: string[] = [];
  let current = "";
  let quote: "\"" | "'" | null = null;
  let tokenStarted = false;

  for (let index = 0; index < value.length; index += 1) {
    const char = value[index];
    const next = value[index + 1];

    if (char === "\\" && next !== undefined && shouldEscapeLaunchArgCharacter(next, quote)) {
      current += next;
      tokenStarted = true;
      index += 1;
      continue;
    }

    if ((char === "\"" || char === "'") && (!quote || quote === char)) {
      quote = quote === char ? null : char;
      tokenStarted = true;
      continue;
    }

    if (!quote && char && /\s/u.test(char)) {
      if (tokenStarted) {
        args.push(current);
        current = "";
        tokenStarted = false;
      }
      continue;
    }

    current += char;
    tokenStarted = true;
  }

  if (tokenStarted) {
    args.push(current);
  }

  return args;
}

function shouldEscapeLaunchArgCharacter(char: string, quote: "\"" | "'" | null): boolean {
  if (quote) {
    return char === quote || char === "\\";
  }

  return char === "\"" || char === "'" || char === "\\" || /\s/u.test(char);
}
