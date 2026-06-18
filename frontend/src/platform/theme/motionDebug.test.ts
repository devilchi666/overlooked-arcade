// Unit tests for the ARC 3 Thrust M motion-diagnostics toggle (D58.9) — default
// OFF, set/read round-trip, and the localStorage persist/clear path. The signal
// is a module singleton, so the tests run in order and reset to OFF at the end.

import { describe, it, expect, vi } from "vitest";
import { motionDebug, motionDebugEnabled, setMotionDebug } from "./motionDebug";

describe("motion diagnostics toggle", () => {
  it("defaults OFF (no persisted value in this runner)", () => {
    expect(motionDebug()).toBe(false);
    expect(motionDebugEnabled()).toBe(false);
  });

  it("round-trips ON → OFF through the setter", () => {
    setMotionDebug(true);
    expect(motionDebug()).toBe(true);
    expect(motionDebugEnabled()).toBe(true);
    setMotionDebug(false);
    expect(motionDebugEnabled()).toBe(false);
  });

  it("persists ON as '1' and clears the key on OFF", () => {
    // jsdom's localStorage stub is partial here, so install a working in-memory
    // one to exercise the real persist path (the module's try/catch keeps
    // persistence best-effort regardless).
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
      setMotionDebug(true);
      expect(mem.get("oa.motionDebug")).toBe("1");
      setMotionDebug(false);
      expect(mem.has("oa.motionDebug")).toBe(false);
    } finally {
      vi.unstubAllGlobals();
      setMotionDebug(false); // leave the singleton OFF for other suites
    }
  });
});
