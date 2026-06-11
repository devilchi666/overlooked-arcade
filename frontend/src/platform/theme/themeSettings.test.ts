// Unit tests for the S5.4 per-theme settings namespace — get/set, fallback,
// collision-free isolation between theme slices, and localStorage persistence.
// The store is a module singleton, so each test uses distinct ids/keys to avoid
// cross-test interference.

import { describe, it, expect, vi } from "vitest";
import { getThemePref, setThemePref } from "./themeSettings";

describe("per-theme settings namespace", () => {
  it("returns the fallback when a theme has no value for a key", () => {
    expect(getThemePref("t-fallback", "missing", 42)).toBe(42);
    expect(getThemePref("t-fallback", "flag", false)).toBe(false);
  });

  it("round-trips a value through set → get", () => {
    setThemePref("t-roundtrip", "compactRows", true);
    expect(getThemePref("t-roundtrip", "compactRows", false)).toBe(true);
    setThemePref("t-roundtrip", "label", "hello");
    expect(getThemePref<string>("t-roundtrip", "label", "")).toBe("hello");
  });

  it("isolates theme slices (one theme's write can't be read as another's)", () => {
    setThemePref("t-alpha", "shared-key", "alpha-value");
    // Same key, different theme → still the fallback (collision-free).
    expect(getThemePref("t-beta", "shared-key", "beta-default")).toBe("beta-default");
    expect(getThemePref<string>("t-alpha", "shared-key", "")).toBe("alpha-value");
  });

  it("persists to localStorage under the namespaced key", () => {
    // The jsdom in this runner ships only a partial localStorage stub, so install
    // a working in-memory one to exercise the real persist() path (the module's
    // own try/catch makes persistence best-effort, so it never breaks the app).
    const mem = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (k: string) => mem.get(k) ?? null,
      setItem: (k: string, v: string) => void mem.set(k, v),
      removeItem: (k: string) => void mem.delete(k),
      clear: () => mem.clear(),
      key: () => null,
      length: 0,
    });
    try {
      setThemePref("t-persist", "k", 7);
      const raw = mem.get("oa.themeSettings");
      expect(raw).toBeTruthy();
      const parsed = JSON.parse(raw!) as Record<string, Record<string, unknown>>;
      expect(parsed["t-persist"]?.k).toBe(7);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
