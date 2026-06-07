# Theming Substrate — Decisions

Append-only log of implementation decisions made during the build.
Strategic decisions made in the planning conversation live in
[docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md)
§3.

---

## 2026-06-06 — Planning decisions (locked)

Captured from the planning conversation that produced
[docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md).

### D1 — One unified premium frontend (no binary split)

OA stays one binary, one window. **No LaunchBox/BigBox-style split
into separate Studio + Launcher apps.** Considered explicitly;
rejected.

**Why:** Couch gamers primary (per [VISION.md](../../VISION.md))
— a split makes every settings change a window-switching ceremony.
Tauri's mental model is one app, one webview, one backend —
splitting would add cross-process SQLite locking + IPC + two
updaters + two installers + doubled CI matrix forever. The 15
shipped SETTINGS categories + per-system drill-in are real work
that would have to be re-ported. LaunchBox/BigBox split for
historical/business reasons that don't apply to OA (LaunchBox
shipped first in 2010; BigBox is a paid $50 add-on).

**How to apply:** When a future contributor proposes splitting
OA into Studio + Launcher, point them here. The architectural
unlock that splitting provides (theme creator scope reduction) is
achieved instead via D2 (engine vs theme territory inside one
window).

### D2 — Two surfaces inside one window (engine vs theme territory)

OA's UI splits into engine territory (always engine-rendered,
visually neutral, summoned via fixed affordance) and theme
territory (where the active `.oatheme` package draws).

**Engine territory:** Settings, Library Manager, Import Wizard,
BIOS pre-checks, Core installer, System Health, Background Jobs.

**Theme territory:** library browsing, game launch ceremony,
now-playing, quick-settings overlay, discovery surfaces.

**Why:** Same scope-reduction benefit BigBox themers get (themes
don't redesign Settings) without splitting the binary. Theme
creators get a tight, achievable scope. The "boring necessary"
parts of OA don't degrade visually under poorly-designed themes.

**How to apply:** Phase 1 of ARC 1 implements this. SURFACES.md
will be the canonical surface-by-surface assignment.

### D3 — Engine summon: fullscreen takeover, top-right corner

When the operator summons the engine surface (Settings et al.),
it fullscreen-takes-over the OA window — not a slide-in drawer,
not a modal overlay, not a separate window. The "summon" icon
themes must reserve lives in the top-right corner. Default
hotkey `F12`; default controller chord `Select+Start`. All three
affordances reach the same engine surface.

**Why:** Fullscreen takeover is the most controller-friendly
presentation (full focus, no overlay-vs-background ambiguity).
Top-right keeps the icon out of typical browsing focus paths.
`F12` matches established convention (RetroArch uses F1).

**How to apply:** Phase 1 implements all three affordances.
Theme manifest's `reserves_corner` field must be `"top-right"`
in ARC 1. Future relaxation (themes pick a corner) deferred
until justified.

### D4 — Theme manifest format: TOML

`theme.toml`, not `theme.json` or `theme.yaml`.

**Why:** Matches `config/systems/<id>/system.yaml` peer format
philosophy (declarative + comment-friendly) without YAML's quirks
(significant whitespace + tag mode confusion). TOML supports
inline comments which JSON doesn't.

**How to apply:** Phase 2 writes the manifest schema + parser.

### D5 — Theme swap requires app restart (ARC 1)

Switching the active theme via Manager → Appearance reloads OA.
**Hot-swap deferred to ARC 3** alongside Theme Studio.

**Why:** Tearing down the active theme's context + remounting
cleanly is non-trivial — gamepad listeners, audio routing, focus
state, all need orchestrated unmount. Shipping that complexity in
ARC 1 risks delaying the substrate launch. Restart is acceptable
UX for a setting an operator changes maybe once a month.

**How to apply:** Phase 5 wires `set_active_theme(id)` to trigger
a Tauri app restart, not a live re-mount.

### D6 — Build-time bundling only (ARC 1)

Themes shipping inside the OA binary or as loose folders in
`<exe_dir>/themes/<id>/`. **Runtime loading from extracted
`.oatheme` zips deferred to ARC 2.**

**Why:** Tauri's `tauri://localhost` origin breaks out-of-bundle
dynamic `import()` without explicit CSP allowlist work. That
CSP work is real and shouldn't gate the substrate launch. Loose
folders are sufficient for dev / dogfood / first wave of
operator-curated themes.

**How to apply:** Phase 5 loader only walks loose folders +
extracts zips for static-bundle inclusion. Dynamic runtime
loading lands when ARC 2 adds scripting (and the CSP work
becomes load-bearing for Rhai sandboxing anyway).

### D7 — Kiosk plan's substrate spec absorbed

The 4-layer model + `.oatheme` zip + federated GitHub Index +
in-engine Theme Studio designed in
[docs/features/kiosk-shell/KIOSK_PLAN.md](../kiosk-shell/KIOSK_PLAN.md)
§2.2-2.5 becomes the substrate for ALL of OA. Kiosk-as-such
(attract mode, multi-monitor, 5-bus mixer) becomes capabilities
the substrate exposes; themes opt in via manifest's
`required_engine_capabilities` field.

**Why:** The Kiosk plan's spec is good. It just isn't actually
kiosk-specific — it's a theming substrate that the Kiosk
implementation happened to design first because that's where the
need was most acute. Building two parallel substrates (one for
desktop, one for kiosk) would diverge fast and create maintenance
debt forever.

**How to apply:** ARCs 2-3 of this plan correspond to KIOSK_PLAN
§2.2 (Rhai + WGSL) and §2.3 (Theme Studio). The kiosk-shell
feature folder stays — it'll eventually hold the Kiosk-mode
specifics (attract mode, multi-monitor, 5-bus mixer) once those
are implemented as substrate capabilities. The 4 reference themes
KIOSK_PLAN §2.5 specs become substrate-level reference themes
rather than kiosk-exclusive.
