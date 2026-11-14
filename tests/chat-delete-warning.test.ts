import { describe, expect, test } from "bun:test";

import {
  createDeleteWarningStore,
  DELETE_WARNING_STORAGE_KEY,
  localDeleteWarningStore,
  type WarningStorage,
} from "../src/chat/delete-warning-store";

class MemoryStorage implements WarningStorage {
  readonly values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

describe("createDeleteWarningStore", () => {
  test("stays unacknowledged until the user explicitly continues", () => {
    const storage = new MemoryStorage();
    const store = createDeleteWarningStore(storage);

    expect(store.acknowledged()).toBe(false);
    store.acknowledge();
    expect(store.acknowledged()).toBe(true);
    expect(storage.getItem(DELETE_WARNING_STORAGE_KEY)).toBe("1");
  });

  test("a new store over the same profile sees the persisted acknowledgement", () => {
    const storage = new MemoryStorage();
    createDeleteWarningStore(storage).acknowledge();

    expect(createDeleteWarningStore(storage).acknowledged()).toBe(true);
  });

  test("null storage never acknowledges and never throws", () => {
    const store = createDeleteWarningStore(null);

    expect(store.acknowledged()).toBe(false);
    expect(() => {
      store.acknowledge();
    }).not.toThrow();
    expect(store.acknowledged()).toBe(false);
  });

  test("read failures count as unacknowledged", () => {
    const storage: WarningStorage = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => undefined,
    };

    expect(createDeleteWarningStore(storage).acknowledged()).toBe(false);
  });

  test("write failures do not crash and are not treated as acknowledged", () => {
    const storage: WarningStorage = {
      getItem: () => null,
      setItem: () => {
        throw new Error("quota");
      },
    };
    const store = createDeleteWarningStore(storage);

    expect(() => {
      store.acknowledge();
    }).not.toThrow();
    expect(store.acknowledged()).toBe(false);
  });
});

describe("localDeleteWarningStore", () => {
  test("works when no browser profile storage exists", () => {
    const store = localDeleteWarningStore();

    expect(store.acknowledged()).toBe(false);
    expect(() => {
      store.acknowledge();
    }).not.toThrow();
  });
});
