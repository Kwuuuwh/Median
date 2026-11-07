import {
  type Envelope,
  type MessageKind,
  PORT,
  PROTOCOL_VERSION,
  type Source,
} from "./protocol";

/// Collector version, injected by the bundler from package.json.
declare const __COLLECTOR_VERSION__: string;

const WARFRAME = 8954;

/// GEP features the collector requests for median.
const FEATURES = ["gep_internal", "game_info", "match_info", "inventory"];

const PING_MS = 15000;
const SNAPSHOT_MS = 30000;
const RETRY_MS = 2500;
const FEATURE_RETRY_MS = 3000;

/// Reconnect delay after median rejects the protocol version; it doubles up to the maximum.
const REFUSED_MS = 5000;
const REFUSED_MAX_MS = 300000;

interface RunningGameInfo {
  classId?: number;
  isRunning?: boolean;
}

interface InfoUpdate {
  feature?: string;
  info?: unknown;
}

let socket: WebSocket | null = null;
let ping: number | null = null;
let snapshots: number | null = null;
let retry: number | null = null;
let refusedFor = 0;
let wired = false;
let ready = false;
/// Whether a feature request is pending, so a second one is not sent on top of it.
let arming = false;
/// Whether median was already notified on this connection that the features are not active.
let told = false;
/// Last feature rejection reason, so a rejection repeated every few seconds is logged once.
let refused: string | null = null;
let running = false;

function now(): number {
  return Date.now();
}

function open(): boolean {
  return socket !== null && socket.readyState === WebSocket.OPEN;
}

function send(kind: MessageKind, rest: Partial<Envelope> = {}): boolean {
  if (!open()) {
    return false;
  }
  const envelope: Envelope = { v: PROTOCOL_VERSION, kind, ts: now(), ...rest };
  (socket as WebSocket).send(JSON.stringify(envelope));
  return true;
}

/// Logs to the console and sends the line to median as a `diag` message.
function log(tag: string, detail?: unknown): void {
  console.log(`median: ${tag}`, detail);
  send("diag", { payload: { tag, detail } });
}

/// Sends one piece of game data to median as an `event`. Every event has the same shape;
/// `feature` names a GEP feature or is absent, and never names the source.
function forward(source: Source, info: unknown, feature?: string): void {
  if (!send("event", { feature, payload: { source, info } })) {
    console.log(`median: dropped ${source}, the link is down`);
  }
}

function every(ms: number, run: () => void): number {
  return setInterval(run, ms) as unknown as number;
}

function stop(timer: number | null): null {
  if (timer !== null) {
    clearInterval(timer);
  }
  return null;
}

/// Fetches all current game info and forwards it. While the features are not active, it reports
/// that once per connection and requests them again, because Overwolf does not always report a
/// failure, and staying silent would leave the session without game data.
function snapshot(): void {
  if (!open()) {
    return;
  }
  if (!ready) {
    if (!told) {
      told = true;
      log("the game-event feeds are not up");
    }
    if (!arming) {
      requireFeatures();
    }
    return;
  }
  overwolf.games.events.getInfo((info) => forward("poll", info));
}

function later(ms: number): void {
  if (retry !== null) {
    return;
  }
  retry = setTimeout(() => {
    retry = null;
    connect();
  }, ms) as unknown as number;
}

function connect(): void {
  const ws = new WebSocket(`ws://127.0.0.1:${PORT}`);
  socket = ws;

  ws.onopen = () => {
    send("hello", {
      payload: { collector: "median-gep", version: __COLLECTOR_VERSION__ },
    });
    told = false;
    ping = every(PING_MS, () => send("ping"));
    snapshots = every(SNAPSHOT_MS, snapshot);
    snapshot();
  };

  ws.onmessage = (message) => {
    let envelope: Envelope;
    try {
      envelope = JSON.parse(String(message.data)) as Envelope;
    } catch {
      return;
    }
    if (envelope.kind === "error" && envelope.reason === "version") {
      refusedFor = refusedFor === 0 ? REFUSED_MS : Math.min(refusedFor * 2, REFUSED_MAX_MS);
      console.log(`median: refused the protocol version, retrying in ${refusedFor} ms`);
      ws.close();
      return;
    }
    if (envelope.kind === "welcome") {
      refusedFor = 0;
    }
  };

  ws.onclose = () => {
    socket = null;
    ping = stop(ping);
    snapshots = stop(snapshots);
    if (running) {
      later(refusedFor === 0 ? RETRY_MS : refusedFor);
    }
  };

  ws.onerror = () => ws.close();
}

function onInfoUpdate(update: InfoUpdate): void {
  forward("info", update.info, update.feature);
}

function onNewEvents(events: unknown): void {
  forward("events", events);
}

/// Requests the features until they are active, while the game runs: the GEP provider attaches
/// to the game some time after launch, and giving up would leave the session without game data.
function requireFeatures(): void {
  if (!running) {
    arming = false;
    return;
  }
  arming = true;
  overwolf.games.events.setRequiredFeatures(FEATURES, (result) => {
    if (result.success) {
      arming = false;
      ready = true;
      refused = null;
      log("the game-event feeds are up");
      snapshot();
      return;
    }
    const reason = String(result.error ?? "no reason given");
    if (reason !== refused) {
      refused = reason;
      log("the game-event feeds were refused", reason);
    }
    setTimeout(requireFeatures, FEATURE_RETRY_MS);
  });
}

/// Adds the GEP listeners once per game session and requests the features.
function wire(): void {
  if (wired) {
    return;
  }
  wired = true;
  overwolf.games.events.onInfoUpdates2.removeListener(onInfoUpdate);
  overwolf.games.events.onInfoUpdates2.addListener(onInfoUpdate);
  overwolf.games.events.onNewEvents.removeListener(onNewEvents);
  overwolf.games.events.onNewEvents.addListener(onNewEvents);
  requireFeatures();
}

function unwire(): void {
  overwolf.games.events.onInfoUpdates2.removeListener(onInfoUpdate);
  overwolf.games.events.onNewEvents.removeListener(onNewEvents);
  overwolf.games.events.setRequiredFeatures([], () => {});
  wired = false;
  ready = false;
}

function isWarframe(info: RunningGameInfo | null | undefined): boolean {
  return !!info && info.classId === WARFRAME;
}

function started(): void {
  running = true;
  wire();
  if (socket === null && retry === null) {
    connect();
  }
}

/// Closes the connection and stops reconnecting until Warframe runs again.
function stopped(): void {
  running = false;
  if (retry !== null) {
    clearTimeout(retry);
    retry = null;
  }
  unwire();
  snapshots = stop(snapshots);
  const ws = socket;
  socket = null;
  ws?.close();
}

overwolf.games.getRunningGameInfo((info) => {
  if (isWarframe(info) && info.isRunning === true) {
    started();
  }
});

overwolf.games.onGameInfoUpdated.addListener((event) => {
  if (!isWarframe(event.gameInfo)) {
    return;
  }
  if (event.gameInfo?.isRunning === true) {
    started();
  } else {
    stopped();
  }
});
