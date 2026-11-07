import recorded from "virtual:median-snapshot";
import { events } from "@median/ipc";
import { standActions } from "./actions";
import { servePictures } from "./pictures";
import { snapshotAnswers } from "./snapshot";
import { installStand } from "./stand";
import { worldAsOf } from "./world";

const state = recorded.getState?.[0]?.answer;
if (state?.worldstate)
  state.worldstate = worldAsOf(state.worldstate, Math.floor(Date.now() / 1000));

/** Riven policy fields missing from the recorded snapshot, set to the backend's defaults. */
const RIVEN_POLICY = {
  rivens: false,
  rankUp: false,
  rivenCapital: 0,
  rivenShare: 0,
  rivenMargin: 25,
  rivenLeastProfit: 50,
};
if (state) state.policy = { ...RIVEN_POLICY, ...state.policy };

/** Fields a snapshot recorded by an older build lacks. */
if (state) {
  state.game = {
    ...state.game,
    runningSince: state.game.runningSince ?? null,
    recognition: state.game.recognition ?? "ready",
  };
  state.updateChannel = state.updateChannel ?? null;
  if (state.collector.state === "live") {
    state.collector = { ...state.collector, feeds: state.collector.feeds ?? { state: "up" } };
  }
}
for (const call of recorded.getSets ?? []) {
  for (const set of call.answer) set.per_set = set.per_set ?? set.members.map(() => 1);
}

/** With `?troubles` in the URL, the mock reports a problem with every source. */
if (state && new URLSearchParams(location.search).has("troubles")) {
  state.collector = { state: "outdated", speaks: 2 };
  state.game = { ...state.game, listening: true, shared: true, recognition: "failed" };
  state.market = { ...state.market, failingSince: Math.floor(Date.now() / 1000) - 240 };
  state.tasks = [{ name: "market", state: { kind: "restarting", reason: "the socket closed" } }];
}

installStand({ ...snapshotAnswers(recorded), ...standActions(recorded) });
servePictures();

/** Interval in milliseconds at which the mock re-emits the world state, like the backend's poll. */
const WORLD_POLL_MS = 30_000;

setInterval(() => {
  if (!state?.worldstate) return;
  state.worldstate = { ...state.worldstate, at: Math.floor(Date.now() / 1000) };
  void events.worldChanged.emit({ worldstate: state.worldstate, shelves: state.shelves });
}, WORLD_POLL_MS);

/** Number of watched items in the mock, as counted in the live app on 08.10.2026. */
const HOT = 313;

/** Duration in milliseconds of the mock's `counting` phase before fetching starts. */
const COUNTING_MS = 2_000;

/** Milliseconds between fetch progress steps: about three items a second, like the scheduler. */
const WARM_STEP_MS = 400;

/** After the counting phase, the mock fetches watched items step by step, like the scheduler. */
if (state) {
  state.agent = { ...state.agent, warm: 0, hot: 0 };
  setTimeout(() => {
    const warming = setInterval(() => {
      const warm = Math.min(HOT, state.agent.warm + 1 + Math.floor(Math.random() * 2));
      state.agent = { ...state.agent, warm, hot: HOT };
      void events.agentChanged.emit({ agent: state.agent });
      if (warm === HOT) clearInterval(warming);
    }, WARM_STEP_MS);
  }, COUNTING_MS);
}
