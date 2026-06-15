# External Emulator Depth — decisions

Append-only. Decision ids are **ED**-prefixed to stay distinct from the
archived Phase-C launcher-abstraction D1–D5 (which cover the same subject
area, in [_archive/PLANS/launcher-abstraction.md](../../_archive/PLANS/launcher-abstraction.md)).

---

## ED1 — Extended control = OA-authored adapters, NOT a third-party plugin SDK (2026-06-15)

**Decision:** Deeper per-emulator control (config injection, per-game
config, screenshots, eventual window-wrapping) is implemented as
**OA-authored adapters maintained in-tree**, declared via recipe
capability flags above `ExternalProcessLauncher`. OA does **not** ship a
generic plugin format/SDK for third-party developers.

**Why:** A generic SDK relitigates the 2026-06-02 PARKING_LOT
plugin-API rejection — only the narrow "operator points OA at additional
emulator profiles" case was un-parked (2026-06-03). A public plugin
contract carries versioned-stable-API + security + maintenance burden
that doesn't fit a one-person non-commercial gift project. The adapter
model is the natural extension of the shipped `Launcher` trait +
`LauncherCapabilities` and keeps everything under OA's control.

**Operator's framing:** "I don't mind A" — with the caveat captured in
ED2.

---

## ED2 — Per-emulator knowledge is updatable data, decoupled from the OA binary (2026-06-15)

**Decision:** All per-emulator launch/control knowledge lives in the
`config/emulators/<id>.yaml` recipe files (data), refreshable
**independently of the OA binary** through the operator-initiated
content-pack-style update channel ([content-packs.md](../../PLANS/content-packs.md)).
A changed emulator flag = a published recipe update = a user "update
recipes" click — **no OA rebuild/reship.** Compiled Rust stays a thin
generic engine interpreting declarative recipe data; declarative-first
with a code escape hatch. Only a genuinely-new control *mechanism* needs
a code change + release.

**Why (operator, verbatim intent):** "emulators change cli and options
all the time so we need a way to update without having to constantly
update the whole program." This is the load-bearing constraint that
shapes the whole arc — it converts ED1's "OA-authored" from
"hardcoded-in-the-binary" to "OA-authored recipes shipped as updatable
data."

**How to apply:** Keep new per-emulator behavior expressible in the YAML
recipe wherever possible. Before adding a hardcoded Rust branch for a
specific emulator, ask whether it can be a declarative recipe field
instead. Cross-ref the theming "low floor / high ceiling, declarative-
first + escape hatch" philosophy.

---

## ED3 — Install pipeline has a per-emulator legal gate; default Yellow (2026-06-15)

**Decision:** The "install this emulator for me" pipeline classifies each
emulator **Green** (license clearly permits OA to download + install it)
or **Yellow** (OA may only link to the official download; the user
installs it themselves). OA **never** auto-downloads a Yellow emulator.
**Unverified licenses default to Yellow.** The absolute rule stands: zero
ROMs, zero BIOS, zero keys — ever.

**Why (operator):** "we need to make sure the emulators we offer to
download through OA actually legally allow us to download them and install
them in our frontend. I dont want to break someones trust." DuckStation
(CC BY-NC-ND, no repackaging) is the canonical Yellow case; firmware/keys
(PS3UPDAT.PUP, `keys.txt`) are user-installed preconditions OA
detect-and-prompts for, never provides.

---

## ED4 — Schema accretion, not one-profile-per-pair; `--system` is a reserved fallback (2026-06-15)

**Decision:** Extend the recipe schema additively. ares + BizHawk
**auto-detect the system from the game file** (verified 2026-06-15
against ares README/docs + the BizHawk wiki), so the per-system argument
problem mostly dissolves — both get a single positional `{content}`
recipe like the other emulators. The genuinely-needed additions are a
**per-OS `binary_name` map** and **MAME's non-path content model**. The
optional `--system` override is a **reserved seam**, documented but not
built until a real ambiguous-extension case surfaces.

**Why:** Verifying the "may auto-detect" flag from the research doc
collapsed the biggest feared schema change. ares `--system` is
explicitly "useful when the system type cannot be auto-detected"
(optional); BizHawk maps file extension → console on load. One-profile-
per-(emulator,system)-pair (~20 near-dup files) is avoided.

**Cross-ref:** research doc open schema question #1 (now resolved);
the per-OS binary table in `RESEARCH/external-emulators.md`.

---

## ED5 — Window-wrapping / embedding is the north star, deferred to its own arc (2026-06-15)

**Decision:** Running the emulator's output *inside* OA's window
(seamless single-app feel) is the long-term ambition of this arc, but is
**deferred to its own focused arc**, proven one emulator at a time. The
near-term recipe + install + control foundation must **not preclude** it.

**Why (operator):** "I am hoping down the road to add advanced features
like actually catching and wrapping emulators into our own window... this
is a issue almost all frontends have and never do well." It's genuinely
hard and OS-fragile (window reparenting/capture breaks differently per
emulator) — under-promise, earn it carefully, rather than claim it
everywhere and ship something flaky (the trap other frontends fall into).

---

## ED6 — Control capabilities are a separate axis from the Phase-C D5 LauncherCapabilities (2026-06-15)

**Decision:** The new "extended control" capability surface (config
injection, per-game config, artifact reading, eventual embedding) is a
**separate namespace** from the Phase-C `LauncherCapabilities` (D5 in the
archived launcher plan). D5 = which of OA's *own* QuickSettings an
external launcher exposes (rewind/savestate/run_ahead/input_remap — all
false for externals today). The control surface = OA driving the
*emulator's own* config/state. Do not overload D5's flags.

**Why:** The two are orthogonal concerns. Conflating them would make
"external supports rewind" (false) and "OA can inject a config file"
(true) share a flag set, muddying both. Keeping them separate keeps each
capability check honest.
