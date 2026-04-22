export function encodeWorkspaceWebSocketPayload(value: unknown): string {
  return JSON.stringify(value, (_key, candidate) => {
    if (typeof candidate === "bigint") {
      return {
        $bigint: candidate.toString(),
      };
    }

    return candidate;
  });
}

export function decodeWorkspaceWebSocketPayload<T>(raw: string): T {
  return JSON.parse(raw, (_key, candidate) => {
    if (
      candidate
      && typeof candidate === "object"
      && "$bigint" in candidate
      && typeof candidate.$bigint === "string"
    ) {
      return BigInt(candidate.$bigint);
    }

    return candidate;
  }) as T;
}
