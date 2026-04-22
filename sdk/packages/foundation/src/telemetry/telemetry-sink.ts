export interface TelemetryEvent {
  name: string;
  attributes?: Record<string, string | number | boolean | null | undefined>;
}

export interface TelemetrySink {
  emit(event: TelemetryEvent): void;
}

export const noopTelemetrySink: TelemetrySink = {
  emit() {},
};
