import { commands } from "@median/ipc";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import type { Answers } from "./answers";
import { installStand } from "./stand";

afterEach(clearMocks);

describe("installStand", () => {
  it("hands a command its stand-in's answer, arguments in order", async () => {
    installStand({ getOtherName: (lang, item) => `${lang}:${item}` });

    await expect(commands.getOtherName("en", "/Lotus/Rhino")).resolves.toEqual({
      status: "ok",
      data: "en:/Lotus/Rhino",
    });
  });

  it("refuses a command the stand has no answer for", async () => {
    installStand({});

    await expect(commands.syncCatalog()).resolves.toEqual({
      status: "error",
      error: "sync_catalog has no answer on the stand",
    });
  });

  it("holds each stand-in to its command's answer type", () => {
    const answers: Answers = {
      // @ts-expect-error a build is not a number
      getBuild: () => 1,
    };
    expect(answers.getBuild).toBeTypeOf("function");
  });
});
