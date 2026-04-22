export class GenerationToken {
  #current = 0;

  current(): number {
    return this.#current;
  }

  next(): number {
    this.#current += 1;
    return this.#current;
  }

  isCurrent(token: number): boolean {
    return token === this.#current;
  }
}
