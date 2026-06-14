# Settings IA Redesign — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-14 — Planning session (design locked, execution deferred)

- **Shipped:** the design. Explored the current Settings → Library surface
  (`engine/SettingsSections.tsx::LibrarySettings` → `LibraryManagerPage` 3-tab
  manager) and confirmed the key finding — today's Settings → Library is ~100%
  *management*; appearance (`sortKey`/`groupBy`/`viewMode`) is wired into
  `LibraryView` but has **no UI** (only tile-size, via `GridControls`), and is
  theme territory per SURFACES/D19. Operator re-cut the whole Settings IA around
  user intent: new top-level **Themes/Appearance · Library · Organize My
  Collection · Import & Setup · External Emulators** groups. Decisions D1–D7
  locked (verify on re-point; declarative per-theme settings schema; start at the
  re-skeleton). Plan written to
  [../../PLANS/settings-ia-redesign.md](../../PLANS/settings-ia-redesign.md);
  feature folder + NEXT.md Slice 1 + ACTIVE_WORK + INDEX updated.
- **Almost:** nothing built — planning only.
- **Next:** **Slice 1 — IA re-skeleton + Library/Organize split** (frontend-only;
  `engine/SettingsPanel.tsx` groups/categories + split `LibraryManagerPage` into
  Library + Organize card hubs reusing `engine/systemsHub/` primitives). Queued
  in NEXT.md HIGH band.
