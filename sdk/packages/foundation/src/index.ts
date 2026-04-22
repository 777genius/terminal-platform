export {
  createExternalStore,
  type ExternalStore,
  type StoreListener,
} from "./store/external-store.js";

export { toDisposable, type Disposable, type DisposeCallback } from "./lifecycle/disposable.js";
export { ResourceScope } from "./lifecycle/resource-scope.js";

export { AsyncLane } from "./async/async-lane.js";
export { GenerationToken } from "./async/generation-token.js";

export { BasePlatformError } from "./errors/base-error.js";

export {
  noopTelemetrySink,
  type TelemetryEvent,
  type TelemetrySink,
} from "./telemetry/telemetry-sink.js";
