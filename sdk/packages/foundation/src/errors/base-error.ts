export class BasePlatformError extends Error {
  readonly code: string;
  override readonly cause?: unknown;

  constructor(code: string, message: string, cause?: unknown) {
    super(message);
    this.name = "BasePlatformError";
    this.code = code;
    this.cause = cause;
  }
}
