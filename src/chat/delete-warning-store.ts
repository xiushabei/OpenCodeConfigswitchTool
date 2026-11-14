/**
 * Versioned acknowledgement of the chat-delete test-phase warning.
 *
 * `localStorage` belongs to one local profile, so this key is naturally scoped per profile.
 * Bump the version in the key to show the warning again after its text or behaviour changes.
 */
export const DELETE_WARNING_STORAGE_KEY = "omo.chat-delete-warning.v1";

export type WarningStorage = Pick<Storage, "getItem" | "setItem">;

export interface DeleteWarningStore {
  acknowledged(): boolean;
  acknowledge(): void;
}

function readStorage(): WarningStorage | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const storage: Storage | undefined = window.localStorage;
    return storage ?? null;
  } catch {
    // Storage can be denied in hardened profiles; the warning must still work.
    return null;
  }
}

/** Pure policy over injected storage; unavailable storage never acknowledges anything. */
export function createDeleteWarningStore(
  storage: WarningStorage | null,
): DeleteWarningStore {
  return {
    acknowledged: () => {
      if (storage === null) {
        return false;
      }
      try {
        return storage.getItem(DELETE_WARNING_STORAGE_KEY) === "1";
      } catch {
        return false;
      }
    },
    acknowledge: () => {
      if (storage === null) {
        return;
      }
      try {
        storage.setItem(DELETE_WARNING_STORAGE_KEY, "1");
      } catch {
        // A failed write stays unacknowledged so the warning is shown again next time.
      }
    },
  };
}

export function localDeleteWarningStore(): DeleteWarningStore {
  return createDeleteWarningStore(readStorage());
}
