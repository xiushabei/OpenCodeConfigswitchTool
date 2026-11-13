import { expect, test } from "bun:test";

import {
  ensureCatalogueReady,
  getCatalogue,
  loadCatalogue,
} from "../src/models/catalogue";
import type { ModelOption } from "../src/models/types";

const MODELS: readonly ModelOption[] = [
  { id: "openai/gpt-5", provider: "openai", model: "gpt-5" },
  { id: "anthropic/claude", provider: "anthropic", model: "claude" },
];

interface PendingCall {
  resolve(value: unknown): void;
  reject(reason: unknown): void;
}

const calls: { command: string; refresh: boolean }[] = [];
const queued: PendingCall[] = [];

function queueCall(): PendingCall {
  const call: PendingCall = {
    resolve: () => {},
    reject: () => {},
  };
  queued.push(call);
  return call;
}

const runtime = globalThis as unknown as {
  window: unknown;
  __TAURI_INTERNALS__: unknown;
};
runtime.window = globalThis;
runtime.__TAURI_INTERNALS__ = {
  invoke: (command: string, args?: Record<string, unknown>) => {
    calls.push({ command, refresh: args?.["refresh"] === true });
    const call = queued.shift();
    if (call === undefined) {
      return Promise.reject(new Error("unexpected model catalogue call"));
    }
    return new Promise<unknown>((resolve, reject) => {
      call.resolve = resolve;
      call.reject = reject;
    });
  },
};

test("catalogue request is shared, cached when ready, and retried with refresh", async () => {
  expect(getCatalogue().status).toBe("idle");

  const firstCall = queueCall();
  const first = loadCatalogue(false);
  const second = loadCatalogue(false);
  const awaited = ensureCatalogueReady();
  expect(second).toBe(first);
  expect(awaited).toBe(first);
  expect(calls).toHaveLength(1);
  expect(calls[0]).toEqual({ command: "list_models", refresh: false });
  expect(getCatalogue().status).toBe("loading");

  firstCall.resolve({ models: MODELS, directory: "D:/qa/models" });
  await first;
  expect(getCatalogue().status).toBe("ready");
  expect(getCatalogue().models).toHaveLength(2);

  await ensureCatalogueReady();
  expect(calls).toHaveLength(1);

  const failure = queueCall();
  const failed = loadCatalogue(true);
  expect(calls).toHaveLength(2);
  failure.reject(new Error("CLI 读取失败"));
  await failed;
  expect(getCatalogue().status).toBe("error");
  expect(getCatalogue().error).toContain("CLI 读取失败");

  const retryCall = queueCall();
  const retried = ensureCatalogueReady();
  expect(calls).toHaveLength(3);
  expect(calls[2]).toEqual({ command: "list_models", refresh: true });
  retryCall.resolve({
    models: MODELS.slice(0, 1),
    directory: "D:/qa/recovered",
  });
  await retried;
  expect(getCatalogue().status).toBe("ready");
  expect(getCatalogue().directory).toBe("D:/qa/recovered");
  await ensureCatalogueReady();
  expect(calls).toHaveLength(3);
});
