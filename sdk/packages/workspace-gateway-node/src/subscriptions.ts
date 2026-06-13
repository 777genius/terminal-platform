import type { WebSocket } from "ws";
import {
  decodeWorkspaceWebSocketPayload,
  encodeWorkspaceWebSocketPayload,
} from "@terminal-platform/workspace-adapter-websocket";
import type {
  WorkspaceGatewayStreamClientMessage,
  WorkspaceGatewayStreamServerMessage,
} from "@terminal-platform/workspace-adapter-websocket/protocol";
import type { WorkspaceSubscription } from "@terminal-platform/workspace-contracts";

import { createGatewayErrorEnvelope, createSubscriptionError } from "./errors.js";
import type {
  WorkspaceGatewayCloseReason,
  WorkspaceGatewayFaultInjectionPort,
  WorkspaceGatewayLogger,
  WorkspaceRuntimeClientPort,
} from "./types.js";
import { validateStreamClientMessage } from "./validation.js";

interface SubscriptionRecord {
  readonly subscriptionId: string;
  readonly sessionId: string;
  disposed: boolean;
  subscription: WorkspaceSubscription | null;
  pump: Promise<void> | null;
}

const DEFAULT_SUBSCRIPTION_CLOSE_TIMEOUT_MS = 250;

export class WorkspaceGatewayStreamConnection {
  readonly #socket: WebSocket;
  readonly #runtime: WorkspaceRuntimeClientPort;
  readonly #subscriptions = new Map<string, SubscriptionRecord>();
  readonly #logger: WorkspaceGatewayLogger;
  readonly #faultInjection: WorkspaceGatewayFaultInjectionPort | null;
  readonly #closeTimeoutMs: number;
  #disposed = false;

  constructor(options: {
    readonly socket: WebSocket;
    readonly runtime: WorkspaceRuntimeClientPort;
    readonly logger?: WorkspaceGatewayLogger;
    readonly faultInjection?: WorkspaceGatewayFaultInjectionPort | null;
    readonly closeTimeoutMs?: number;
  }) {
    this.#socket = options.socket;
    this.#runtime = options.runtime;
    this.#logger = options.logger ?? {};
    this.#faultInjection = options.faultInjection ?? null;
    this.#closeTimeoutMs = options.closeTimeoutMs ?? DEFAULT_SUBSCRIPTION_CLOSE_TIMEOUT_MS;
  }

  handlePayload(raw: string): void {
    void this.handlePayloadAsync(raw);
  }

  async dispose(reason: WorkspaceGatewayCloseReason = "dispose"): Promise<void> {
    if (this.#disposed) {
      return;
    }

    this.#disposed = true;
    const records = [...this.#subscriptions.values()];
    this.#subscriptions.clear();
    await Promise.allSettled(records.map((record) => this.closeRecord(record, reason)));
  }

  private async handlePayloadAsync(raw: string): Promise<void> {
    let message: WorkspaceGatewayStreamClientMessage;

    try {
      message = validateStreamClientMessage(decodeWorkspaceWebSocketPayload<unknown>(raw));
    } catch (error) {
      this.closeSocket("Invalid stream message", error);
      return;
    }

    if (message.type === "workspace_subscribe") {
      await this.subscribe(message);
      return;
    }

    await this.unsubscribe(message.subscriptionId);
  }

  private async subscribe(
    message: Extract<WorkspaceGatewayStreamClientMessage, { type: "workspace_subscribe" }>,
  ): Promise<void> {
    if (this.#disposed) {
      return;
    }

    if (this.#subscriptions.has(message.subscriptionId)) {
      await this.send({
        type: "workspace_subscription_rejected",
        subscriptionId: message.subscriptionId,
        error: createGatewayErrorEnvelope(createSubscriptionError("subscriptionId is already active")),
      });
      return;
    }

    const record: SubscriptionRecord = {
      subscriptionId: message.subscriptionId,
      sessionId: message.sessionId,
      disposed: false,
      subscription: null,
      pump: null,
    };
    this.#subscriptions.set(record.subscriptionId, record);

    try {
      await this.#faultInjection?.beforeSubscriptionOpen?.(message);
      const subscription = await this.#runtime.openSubscription(message.sessionId, message.spec);
      if (record.disposed || this.#subscriptions.get(record.subscriptionId) !== record) {
        await this.closeSubscription(subscription);
        return;
      }

      record.subscription = subscription;
      await this.send({
        type: "workspace_subscription_ack",
        subscriptionId: record.subscriptionId,
        meta: subscription.meta(),
      });
      record.pump = this.pump(record);
    } catch (error) {
      this.#subscriptions.delete(record.subscriptionId);
      record.disposed = true;
      await this.send({
        type: "workspace_subscription_rejected",
        subscriptionId: record.subscriptionId,
        error: createGatewayErrorEnvelope(error),
      });
    }
  }

  private async unsubscribe(subscriptionId: string): Promise<void> {
    const record = this.#subscriptions.get(subscriptionId);
    if (!record) {
      await this.sendClosed(subscriptionId);
      return;
    }

    this.#subscriptions.delete(subscriptionId);
    await this.closeRecord(record, "unsubscribe");
    await this.sendClosed(subscriptionId);
  }

  private async pump(record: SubscriptionRecord): Promise<void> {
    const subscription = record.subscription;
    if (!subscription) {
      return;
    }

    try {
      while (!record.disposed && this.#subscriptions.get(record.subscriptionId) === record) {
        const event = await subscription.nextEvent();
        if (!event) {
          break;
        }

        if (record.disposed || this.#subscriptions.get(record.subscriptionId) !== record) {
          return;
        }

        await this.send({
          type: "workspace_subscription_event",
          subscriptionId: record.subscriptionId,
          event,
        });
      }
    } catch (error) {
      if (!record.disposed && this.#subscriptions.get(record.subscriptionId) === record) {
        await this.send({
          type: "workspace_subscription_error",
          subscriptionId: record.subscriptionId,
          error: createGatewayErrorEnvelope(error),
        });
      }
    } finally {
      if (!record.disposed && this.#subscriptions.get(record.subscriptionId) === record) {
        this.#subscriptions.delete(record.subscriptionId);
        await this.closeRecord(record, "runtime_closed");
        await this.sendClosed(record.subscriptionId);
      }
    }
  }

  private async closeRecord(record: SubscriptionRecord, reason: WorkspaceGatewayCloseReason): Promise<void> {
    if (record.disposed) {
      return;
    }

    record.disposed = true;
    const subscription = record.subscription;
    record.subscription = null;
    if (!subscription) {
      return;
    }

    try {
      await this.#faultInjection?.beforeSubscriptionClose?.(record.subscriptionId);
      await this.closeSubscription(subscription);
    } catch (error) {
      this.#logger.warn?.("workspace gateway subscription close failed", {
        reason,
        subscriptionId: record.subscriptionId,
        error,
      });
    }
  }

  private async closeSubscription(subscription: WorkspaceSubscription): Promise<void> {
    await withTimeout(subscription.close(), this.#closeTimeoutMs);
  }

  private async sendClosed(subscriptionId: string): Promise<void> {
    await this.send({
      type: "workspace_subscription_closed",
      subscriptionId,
    });
  }

  private async send(message: WorkspaceGatewayStreamServerMessage): Promise<void> {
    if (this.#disposed) {
      return;
    }

    try {
      await this.#faultInjection?.beforeServerSend?.(message);
      this.#socket.send(encodeWorkspaceWebSocketPayload(message));
    } catch (error) {
      this.closeSocket("Workspace gateway stream send failed", error);
      await this.dispose("send_failure");
    }
  }

  private closeSocket(reason: string, error?: unknown): void {
    this.#logger.warn?.(reason, error ? { error } : undefined);
    try {
      this.#socket.close(1011, reason);
    } catch {
      this.#socket.terminate();
    }
  }
}

async function withTimeout(promise: Promise<void>, timeoutMs: number): Promise<void> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    await Promise.race([
      promise,
      new Promise<void>((resolve) => {
        timer = setTimeout(resolve, timeoutMs);
      }),
    ]);
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
  }
}
