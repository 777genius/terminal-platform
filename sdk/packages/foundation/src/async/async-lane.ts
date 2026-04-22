export class AsyncLane {
  #tail: Promise<unknown> = Promise.resolve();

  enqueue<TResult>(operation: () => Promise<TResult>): Promise<TResult> {
    const run = this.#tail.then(operation, operation);
    this.#tail = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }
}
