import type { Release } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { downloadedShare, progressed } from "./updater";

const release: Release = { revision: 120, channel: "stable", changes: [], published: null };

describe("progressed", () => {
  it("keeps downloading until the whole download is in", () => {
    expect(progressed(release, { downloaded: 400, total: 1000 })).toEqual({
      step: "downloading",
      release,
      downloaded: 400,
      total: 1000,
    });
  });

  it("turns to installing once the last byte arrives", () => {
    expect(progressed(release, { downloaded: 1000, total: 1000 })).toEqual({
      step: "installing",
      release,
    });
  });

  it("stays downloading while the size is unknown", () => {
    expect(progressed(release, { downloaded: 5000, total: null }).step).toBe("downloading");
  });
});

describe("downloadedShare", () => {
  it("is the received part of a known size", () => {
    expect(downloadedShare({ step: "downloading", release, downloaded: 250, total: 1000 })).toBe(
      0.25,
    );
  });

  it("is unknown without a size or outside a download", () => {
    expect(
      downloadedShare({ step: "downloading", release, downloaded: 250, total: null }),
    ).toBeNull();
    expect(downloadedShare({ step: "latest" })).toBeNull();
  });
});
