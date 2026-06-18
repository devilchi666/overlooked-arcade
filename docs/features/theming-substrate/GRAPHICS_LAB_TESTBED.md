# Graphics Lab — the strip-on-ship testbed theme

**What it is.** `themes/lab/` is a built-in, **experimental** theme that serves as
the permanent home for in-flight graphical work — motion (ARC 3 Thrust M) first,
then shaders / video / attract. It is a navigable shell (routes/tabs + a real game
grid) so it can dogfood view-transitions, selection choreography, ambient loops, and
box-art treatments on a real surface (DECISIONS **D55** — dogfood motion on a
navigable surface, not the single-surface `DeclarativeShell`).

**Why it exists instead of the F10 dev bench.** The `frontend/src/dev/` bench
(`MotionPlayground` etc., F10) is `import.meta.env.DEV`-gated, so it vanishes from
the `cargo tauri build` release the operator playtests. The lab theme is reachable
in **release** builds (cordoned behind Settings → Experimental), so graphical work
can be validated in the same build the operator actually runs.

**Lifecycle.** Stays in the tree until OA ships a complete product, then strip it.
It is **never** part of the shipped theme set — the normal Appearance picker hides
it (see the `experimental` filter below); the only door is Settings → Experimental →
Graphics Lab → Activate.

---

## The engine vs. the demo — DO NOT strip the engine

Strip-on-ship = the **lab theme** and its access points only. The declarative
**motion model** it exercises is real product code that stays:

| Strip on ship (demo)            | KEEP (product engine)                          |
| ------------------------------- | ---------------------------------------------- |
| `frontend/src/themes/lab/`      | `platform/theme/motion.ts` (resolver + basis)  |
| the four touch-points below     | `platform/theme/manifest.ts` motion types      |
| this doc                        | `platform/theme/ViewTransition.tsx` + presets  |
|                                 | `theme_loader.rs` motion structs               |

Every lab-only line outside `themes/lab/` carries the marker `// [GRAPHICS-LAB]`
so the full strip is one `grep` away.

---

## Strip checklist (the only touch-points outside `themes/lab/`)

Removing the lab is this list, top to bottom:

1. **Delete the folder** — `frontend/src/themes/lab/`.
2. **`frontend/src/themes/index.ts`** — remove the `// [GRAPHICS-LAB]` import line
   and the `lab` entry in `BUILTIN_THEMES`.
3. **`frontend/src/platform/theme/registry.ts`** — `availableThemes()` carries a
   `// [GRAPHICS-LAB]` `experimental` filter. The `experimental?` field on
   `ThemePackage` (`types.ts`) is a generic hidden-theme concept and MAY stay; the
   filter is harmless with no experimental themes registered.
4. **`frontend/src/engine/SettingsSections.tsx`** — remove the `// [GRAPHICS-LAB]`
   "Graphics Lab" `SettingsCard` in `ExperimentalSettings`.
5. **Delete this doc.**

`grep -rn "\[GRAPHICS-LAB\]" frontend/` enumerates 2–4 at any time.

---

## Status log

- **2026-06-17** — Foundation laid on `feat/theming-arc3-motion-model`: the
  `experimental` theme flag, the lab navigable shell skeleton, the Experimental
  launcher, and this strip manifest. Motion model (M-mod.1) lands next.
- **2026-06-18** — Motion model M-mod.1–.4 dogfooded here: Home↔Library view
  transition (`SpecTransition`), selection-choreography hero (spring grow-in +
  staggered title/meta), breathe ambient (`AmbientMotion`), pointer-tilt
  (`useTilt`). The lab now exercises all four audit motion categories. The strip
  checklist is unchanged (still one folder + the same 4 touch-points) — all the new
  engine modules (`motionSpec`/`spring`/`springValue`/`SpecTransition`/
  `AmbientMotion`/`tilt`) are PRODUCT code that stays; only `themes/lab/` strips.
