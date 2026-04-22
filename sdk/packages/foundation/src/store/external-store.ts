export type StoreListener = () => void;

export interface ExternalStore<TSnapshot> {
  getSnapshot(): TSnapshot;
  setSnapshot(snapshot: TSnapshot): void;
  update(updater: (snapshot: TSnapshot) => TSnapshot): void;
  subscribe(listener: StoreListener): () => void;
}

export function createExternalStore<TSnapshot>(initialSnapshot: TSnapshot): ExternalStore<TSnapshot> {
  let snapshot = initialSnapshot;
  const listeners = new Set<StoreListener>();

  const setSnapshot = (nextSnapshot: TSnapshot): void => {
    if (Object.is(snapshot, nextSnapshot)) {
      return;
    }

    snapshot = nextSnapshot;
    for (const listener of listeners) {
      listener();
    }
  };

  return {
    getSnapshot() {
      return snapshot;
    },
    setSnapshot,
    update(updater) {
      setSnapshot(updater(snapshot));
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
