# Deferred: On-Screen Keyboard (controller text entry)

**Status:** Deferred 2026-06-13 (operator chose to defer — option A). Build as
its own slice *after* Slice 2 (engine dialogs). Not a Slice 2 blocker.

## Why deferred, not dropped

Text entry is the one interaction a gamepad can't do natively. Until the OSK
ships, the **physical keyboard is the always-on escape hatch**: a controller-
focused text field is a real DOM `<input>`, and the focus framework already
bails its key handler on `INPUT`/`TEXTAREA`/`SELECT` (`focus.ts`), so typing
flows straight through. Nothing is *broken* without the OSK — text fields are
just keyboard-only. Most Slice 2 dialogs (GameProperties, DiscPicker, Help)
have no text entry at all, so the gap is small and isolated. Deferring costs no
rework because the OSK hooks in centrally (below), so it retro-actively covers
every existing text field the day it lands.

## The design (when we build it)

**Layer 1 — physical keyboard (already works, zero code).** Keep it always
available even after the OSK exists; the OSK is *additive*.

**Layer 2 — controller-driven OSK overlay.** Console-standard (Steam Big
Picture / Xbox / PS): Confirm on a text field pops a navigable keyboard; d-pad/
stick to a key, Confirm to type. v1 = QWERTY grid + Shift/Caps + Backspace +
Space + Done (no prediction — low floor; predictive/symbols layer later = high
ceiling). Reusable `platform/nav` infra, NOT per-dialog code.

**It's "just another control type" in the dispatch.** `useSettingsRowFocusGroup`
already branches Confirm by control: checkbox→flip, select→overlay,
range→adjust, button→click. Add: **`input[type=text|search]` / `textarea` →
open OSK overlay.** Because every Settings row *and* every Slice-2 dialog
navigates through that one hook (via the `Dialog.navigate` upgrade), all text
fields inherit the OSK from this single branch — same DRY payoff as the
select-overlay.

**Writes via the proven pattern.** The OSK mutates the field exactly like the
select-overlay: set `.value`, dispatch a bubbling `input` event so Solid's
binding fires. Take the input gate (`setUiIntercepting(true)`) while open, like
the existing chord-capture flow, so stray keys don't leak.

**Gate auto-open to controller sessions.** Don't pop an OSK at keyboard/mouse
users. Add lightweight **last-input-device tracking** to the nav layer (a
`lastInputWasPad` signal the gamepad bus sets true and key/mouse events set
false); auto-open the OSK only when true. This signal is reusable beyond the
OSK (e.g. showing/hiding glyph hints). The physical keyboard works regardless.

## Scope notes

- Cursor position / mid-string editing: v1 can append + backspace only;
  caret movement is a later enhancement.
- Multi-field forms: Done closes the OSK and returns focus to the row; the
  operator walks to the next field and Confirms again.
- Per-system accent theming + the focus-ring conventions come for free (same
  CSS tokens as the select-overlay).

## Where this is tracked

Referenced from [SLICE_2_PLAN.md](SLICE_2_PLAN.md) (the "recurring limitation"
section) and the SESSION_LOG. Promote to an active slice once Slice 2 lands.
