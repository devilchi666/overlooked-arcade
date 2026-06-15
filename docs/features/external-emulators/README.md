# External Emulators (Depth) — feature stream

Deepening OA's standalone-emulator support beyond "can launch a game":
install pipeline, independently-updatable launch recipes, and
progressive per-emulator control — building toward the long-term north
star of running the emulator inside OA's own window.

## Where things are

- **Plan:** [PLANS/external-emulator-depth.md](../../PLANS/external-emulator-depth.md)
  (phases + slices).
- **Decisions:** [DECISIONS.md](DECISIONS.md) (ED1–ED6).
- **Session log:** [SESSION_LOG.md](SESSION_LOG.md).
- **Research seed:** [RESEARCH/external-emulators.md](../../RESEARCH/external-emulators.md)
  (roster + per-emulator CLI/quirks + per-OS binary table).

## What already shipped (foundation — VL Phase C)

- `oa-core` `Launcher` trait + `LauncherCapabilities`.
- `apps/oa-shell/src/launcher.rs` — `LibretroLauncher` +
  `ExternalProcessLauncher`.
- `apps/oa-shell/src/emulator_profiles.rs` — the
  `config/emulators/<id>.yaml` recipe registry + `emulators.json` /
  `launchers.json` appData prefs.
- Settings → External Emulators surface (Settings-IA Slice 4).
- 9 verified profiles merged (`feat/external-emulator-profiles`).

## The one principle to keep

Per-emulator knowledge is **updatable data** (recipes), refreshable
**without** an OA rebuild (ED2). Compiled Rust stays a thin generic
engine; declarative-first, code escape hatch only for genuinely-new
mechanisms.
