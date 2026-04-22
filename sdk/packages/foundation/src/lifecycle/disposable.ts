export interface Disposable {
  dispose(): void | Promise<void>;
}

export type DisposeCallback = () => void | Promise<void>;

export function toDisposable(dispose: DisposeCallback): Disposable {
  return { dispose };
}
