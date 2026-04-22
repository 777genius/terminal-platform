import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  DiscoveredSession,
  Handshake,
  SavedSessionSummary,
  SessionSummary,
  SubscriptionEvent,
} from "@terminal-platform/runtime-types";

export type WorkspaceObservation =
  | {
      kind: "handshake";
      handshake: Handshake;
    }
  | {
      kind: "sessions";
      sessions: SessionSummary[];
    }
  | {
      kind: "saved_sessions";
      sessions: SavedSessionSummary[];
    }
  | {
      kind: "discovered_sessions";
      backend: DiscoveredSession["route"]["backend"];
      sessions: DiscoveredSession[];
    }
  | {
      kind: "backend_capabilities";
      info: BackendCapabilitiesInfo;
    }
  | {
      kind: "attached_session";
      session: AttachedSession;
    }
  | {
      kind: "subscription_event";
      event: SubscriptionEvent;
    };
