import type { ServiceContext } from "./service-context.js";
import { DEFAULT_WORKSPACE_THEME_ID } from "../read-models/workspace-snapshot.js";

export class ThemeResolutionService {
  readonly #availableThemeIds: ReadonlySet<string>;
  readonly #context: Pick<ServiceContext, "getSnapshot" | "recordDiagnostic" | "updateSnapshot">;

  constructor(
    context: Pick<ServiceContext, "getSnapshot" | "recordDiagnostic" | "updateSnapshot">,
    availableThemeIds: ReadonlySet<string>,
  ) {
    this.#context = context;
    this.#availableThemeIds = availableThemeIds;
  }

  setTheme(themeId: string): void {
    const normalizedThemeId = normalizeThemeId(themeId);
    if (!normalizedThemeId || !this.#availableThemeIds.has(normalizedThemeId)) {
      this.#context.recordDiagnostic({
        code: "theme_unsupported",
        message: normalizedThemeId
          ? `Theme "${normalizedThemeId}" is not registered for this workspace`
          : "Theme id must be a non-empty string",
        severity: "warn",
        recoverable: true,
      });
      return;
    }

    if (this.#context.getSnapshot().theme.themeId === normalizedThemeId) {
      return;
    }

    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      theme: {
        themeId: normalizedThemeId,
      },
    }));
  }
}

export function createAvailableThemeIdSet(themeIds: readonly string[]): Set<string> {
  const availableThemeIds = new Set<string>([DEFAULT_WORKSPACE_THEME_ID]);
  for (const themeId of themeIds) {
    const normalizedThemeId = normalizeThemeId(themeId);
    if (normalizedThemeId) {
      availableThemeIds.add(normalizedThemeId);
    }
  }

  return availableThemeIds;
}

export function normalizeThemeId(themeId: string | null | undefined): string | null {
  const normalizedThemeId = themeId?.trim();
  if (!normalizedThemeId) {
    return null;
  }

  return normalizedThemeId.length > 0 ? normalizedThemeId : null;
}
