/// Version of the collector protocol.
export const PROTOCOL_VERSION = 2;

/// Loopback port median listens on.
export const PORT = 42010;

export type MessageKind =
  | "hello"
  | "welcome"
  | "event"
  | "diag"
  | "ping"
  | "pong"
  | "error";

export type ErrorReason = "version";

/// Source of a piece of game data: a `getInfo` poll, an info update, or new events.
export type Source = "poll" | "info" | "events";

/// Payload of every `event` envelope, for any source.
export interface Feed {
  source: Source;
  /// Game data exactly as Overwolf delivered it.
  info: unknown;
}

export interface Envelope {
  v: number;
  kind: MessageKind;
  feature?: string;
  reason?: ErrorReason;
  ts: number;
  payload?: unknown;
}
