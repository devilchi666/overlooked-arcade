// Controller identity — derive a stable cross-layer device-key from the
// Web Gamepad API `id` string. Phase 0 of the Controller Identity arc.
// See docs/PLANS/controller-identity-substrate.md.
//
// The `id` string format varies across browsers / OS (risk R5):
//   Chrome:   "Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)"
//   Firefox:  "057e-2009-Pro Controller"
//   XInput:   "Xbox 360 Controller (XInput STANDARD GAMEPAD)"   ← no VID/PID
// We parse VID/PID defensively and fall back to a name slug when absent.
//
// The `key` is the cross-layer identity (decision D2 = VID/PID). For a pad
// that reports VID/PID, the Rust gilrs poller derives the IDENTICAL
// `vidpid:<vid>:<pid>` key (see crates/oa-input/src/device_key.rs). For
// XInput pads where the Web `id` omits VID/PID, the layers may diverge —
// the documented R5 limitation, resolved by frontend-as-identity-authority
// (decision (c)): the frontend, which owns profile resolution, is the
// authority and hands Rust the normalized per-port mapping at launch.

export type DeviceIdentity = {
  /** USB vendor id, lowercase 4-hex, or null when the id string omits it. */
  vid: string | null;
  /** USB product id, lowercase 4-hex, or null when the id string omits it. */
  pid: string | null;
  /** Human-readable name with the VID/PID decoration stripped. */
  name: string;
  /** Stable cross-layer key: `vidpid:<vid>:<pid>` or `name:<slug>`. */
  key: string;
};

// "...Vendor: 057e ... Product: 2009..." (Chrome / WebKit). Case-insensitive;
// the `.*?` spans the (sometimes present) text between the two fields.
const VENDOR_PRODUCT_RE = /vendor:\s*([0-9a-f]{4}).*?product:\s*([0-9a-f]{4})/i;
// "057e-2009-Pro Controller" (Firefox).
const FIREFOX_PREFIX_RE = /^([0-9a-f]{4})-([0-9a-f]{4})-(.*)$/i;

/// Parse a Web Gamepad `id` into a stable identity + cross-layer key.
/// Pure and total — never throws; an empty / unrecognized id yields a
/// `name:unknown` key rather than failing.
export function deriveDeviceIdentity(rawId: string): DeviceIdentity {
  const id = (rawId ?? "").trim();

  // Firefox: "vid-pid-name".
  const fx = id.match(FIREFOX_PREFIX_RE);
  if (fx) {
    return mk(fx[1].toLowerCase(), fx[2].toLowerCase(), fx[3].trim() || id);
  }

  // Chrome / WebKit: "...Vendor: vid ...Product: pid...".
  const vp = id.match(VENDOR_PRODUCT_RE);
  if (vp) {
    return mk(vp[1].toLowerCase(), vp[2].toLowerCase(), stripParenthetical(id));
  }

  // No VID/PID anywhere → name fallback (R5).
  return mk(null, null, stripParenthetical(id) || id);
}

function mk(vid: string | null, pid: string | null, name: string): DeviceIdentity {
  const clean = name.trim();
  const key = vid && pid ? `vidpid:${vid}:${pid}` : `name:${slug(clean)}`;
  return { vid, pid, name: clean, key };
}

/// Drop a trailing parenthetical decoration: "Pro Controller (Vendor: …)" →
/// "Pro Controller". Leaves names without a parenthetical untouched.
function stripParenthetical(id: string): string {
  return id.replace(/\s*\([^()]*\)\s*$/, "").trim();
}

/// Lowercase, collapse non-alphanumerics to single dashes, trim dashes.
/// Deterministic so the same pad name always hashes to the same key.
function slug(name: string): string {
  return (
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "unknown"
  );
}
