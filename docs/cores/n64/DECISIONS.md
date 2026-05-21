# n64 Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — Mupen64Plus-Next as default

**Decision:** `mupen64plus_next_libretro.dll` (Mupen64Plus-Next with
GLideN64 video plugin). `parallel_n64_libretro.dll` available as a
per-system alternate.

**Why:** Mupen64Plus-Next is the most-validated libretro N64 core
with the broadest compatibility profile. ParaLLEl-N64 is more
accurate but heavier; operators with strong hardware can swap.

---

## 2026-05-20 — Analog input infra shipped as part of N64 onboarding

**Decision:** The minimal cross-cutting analog input infrastructure
(RETRO_DEVICE_ANALOG dispatch in oa-libretro + gilrs analog polling
in oa-input + `InputState.axes` flow through the emu thread) ships
as part of N64 Phase 0 rather than as standalone Phase 2 work.

**Why:** The N64's analog stick is the PRIMARY movement input for
nearly every N64 game — operator validation without analog support
would require the d-pad-to-analog core option as a permanent
fallback, which is a degraded experience. Operator chose "Ship Phase
0 with analog axes plumbed" over the recommended "digital fallback
only" option during onboarding, accepting the ~1.5x session scope.

The infra is minimal: gamepad LeftStick + RightStick X/Y scale to
i16, pass through `InputState.axes`, store per-port in oa-libretro,
return via cb_input_state on RETRO_DEVICE_ANALOG queries. No
per-system Bindings UI (Phase 2.5) — analog stick maps directly to
libretro analog index 0/1 by convention.

**Considered and rejected:**
- **Defer entirely to Phase 2.** N64 would be unplayable on gamepads
  without analog support; degraded operator experience.
- **Digital fallback only at Phase 0.** Keyboard users get d-pad-to-
  analog hack but gamepad users get nothing — worse trade-off.

---

## 2026-05-20 — Atomic Purple 268° theme (Nintendo home cluster)

**Decision:** `[data-system="n64"]` ships `oklch(0.55 0.22 268)` —
Atomic Purple slotting into the Nintendo home-console violet cluster
(SNES 270° / n64 268° / gamecube 280° / GBA 285°).

**Why:** Period-correct to the iconic 1998 Atomic Purple transparent-
shell N64 variant that became the platform's visual shorthand. The
N64 launch palette was multi-color but post-launch the Atomic Purple
variant + the matching transparent controllers became dominant
nostalgia anchors.

Operator accepted the violet-cluster crowding for Nintendo home-
console visual coherence — same precedent the Saturn 275° deepest-
purple decision used.

---

## 2026-05-20 — 14-button digital + analog stick via axes

**Decision:** `bindings::n64` ships 14 digital entries (d-pad + A/B +
L/R/Z + START + 4 C-buttons). The N64 main analog stick is NOT in
the bit-table; it flows through `InputState.axes[0..2]` (gamepad
LeftStick).

**Why:** The N64 controller has 14 discrete digital buttons + 1
analog stick. Digital buttons go in the bindings module; the analog
stick uses the new cross-cutting analog infra (Phase 0 of this
session). C-buttons stay digital because they ARE discrete buttons on
real hardware (despite their "directional" naming — they're 4
distinct buttons, not an analog stick).

**Considered and rejected:**
- **Treat C-buttons as analog right-stick.** Tempting because it
  mirrors GC C-stick, but N64 hardware has discrete C-buttons. Cores
  expect digital input for them.
