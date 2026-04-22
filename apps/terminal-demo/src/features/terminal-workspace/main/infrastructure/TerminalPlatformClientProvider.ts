import type * as TerminalPlatformSdk from "../../../../../.generated/terminal-platform-node/index.mjs";
import { loadTerminalPlatformSdk } from "./terminal-platform-sdk.js";

export class TerminalPlatformClientProvider {
  readonly #runtimeSlug: string;
  readonly #clientPromise: Promise<TerminalPlatformSdk.TerminalNodeClient>;

  constructor(runtimeSlug: string) {
    this.#runtimeSlug = runtimeSlug;
    this.#clientPromise = this.loadClient();
  }

  async getClient(): Promise<TerminalPlatformSdk.TerminalNodeClient> {
    return this.#clientPromise;
  }

  private async loadClient(): Promise<TerminalPlatformSdk.TerminalNodeClient> {
    const sdk = await loadTerminalPlatformSdk();
    return sdk.TerminalNodeClient.fromRuntimeSlug(this.#runtimeSlug);
  }
}
