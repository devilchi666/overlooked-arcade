// Unit tests for the Phase-0 controller-identity device-key derivation.
// Covers the three Web Gamepad `id` formats (Chrome, Firefox, XInput) plus
// the defensive name-slug fallback (risk R5).

import { describe, it, expect } from "vitest";
import { deriveDeviceIdentity } from "./deviceKey";

describe("deriveDeviceIdentity", () => {
  it("parses the Chrome 'Vendor: … Product: …' form", () => {
    const d = deriveDeviceIdentity(
      "Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)",
    );
    expect(d).toEqual({
      vid: "057e",
      pid: "2009",
      name: "Pro Controller",
      key: "vidpid:057e:2009",
    });
  });

  it("parses the Firefox 'vid-pid-name' form", () => {
    const d = deriveDeviceIdentity("057e-2009-Pro Controller");
    expect(d).toEqual({
      vid: "057e",
      pid: "2009",
      name: "Pro Controller",
      key: "vidpid:057e:2009",
    });
  });

  it("normalizes uppercase hex to lowercase (cross-layer stability)", () => {
    expect(deriveDeviceIdentity("Pad (Vendor: 057E Product: 2009)").key).toBe(
      "vidpid:057e:2009",
    );
    expect(deriveDeviceIdentity("057E-2009-Pad").key).toBe("vidpid:057e:2009");
  });

  it("falls back to a name slug when XInput omits VID/PID (R5)", () => {
    const d = deriveDeviceIdentity("Xbox 360 Controller (XInput STANDARD GAMEPAD)");
    expect(d.vid).toBeNull();
    expect(d.pid).toBeNull();
    expect(d.name).toBe("Xbox 360 Controller");
    expect(d.key).toBe("name:xbox-360-controller");
  });

  it("slug collapses punctuation/whitespace and trims dashes", () => {
    expect(deriveDeviceIdentity("Microsoft  X-Box 360 pad!!").key).toBe(
      "name:microsoft-x-box-360-pad",
    );
  });

  it("never throws on empty / unrecognized ids", () => {
    expect(deriveDeviceIdentity("").key).toBe("name:unknown");
    expect(deriveDeviceIdentity("   ").key).toBe("name:unknown");
    // @ts-expect-error — guarding the null/undefined runtime case.
    expect(deriveDeviceIdentity(undefined).key).toBe("name:unknown");
  });

  it("matches the Switch Pro VID/PID identically across both id formats", () => {
    // Same physical pad, two browsers → one stable key (the whole point of D2).
    const chrome = deriveDeviceIdentity("Pro Controller (Vendor: 057e Product: 2009)");
    const firefox = deriveDeviceIdentity("057e-2009-Pro Controller");
    expect(chrome.key).toBe(firefox.key);
    expect(chrome.key).toBe("vidpid:057e:2009");
  });
});
