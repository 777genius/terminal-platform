import type { ServiceContext } from "./service-context.js";

export class ThemeResolutionService {
  readonly #context: Pick<ServiceContext, "updateSnapshot">;

  constructor(context: Pick<ServiceContext, "updateSnapshot">) {
    this.#context = context;
  }

  setTheme(themeId: string): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      theme: {
        themeId,
      },
    }));
  }
}
