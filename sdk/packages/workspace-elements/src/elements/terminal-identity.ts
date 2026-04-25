export interface TerminalEntityIdLabelOptions {
  prefix?: string;
}

export interface TerminalEntityIdLabel {
  label: string;
  title: string;
  isCompact: boolean;
}

export function compactTerminalId(id: string): string {
  if (id.length <= 18) {
    return id;
  }

  return `${id.slice(0, 8)}...${id.slice(-6)}`;
}

export function resolveTerminalEntityIdLabel(
  id: string,
  options: TerminalEntityIdLabelOptions = {},
): TerminalEntityIdLabel {
  const compactId = compactTerminalId(id);
  return {
    label: options.prefix ? `${options.prefix} ${compactId}` : compactId,
    title: id,
    isCompact: compactId !== id,
  };
}
