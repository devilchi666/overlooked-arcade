# Retroverse UI

Experimental top-toolbar IA replacing the legacy sidebar-driven Shell.
Flag-gated behind `Settings → Display → Experimental → Retroverse UI`.

## Design + planning

All design docs live under `docs/PLANS/`:

- **[retroverse-ui-rollout.md](../../PLANS/retroverse-ui-rollout.md)** —
  rollout phase plan + the live "Status surface" + "Remaining work"
  list in §10. Start here.
- [settings-tab-retroverse.md](../../_archive/PLANS/settings-tab-retroverse.md) —
  SETTINGS tab design.
- [play-now-tab-retroverse.md](../../_archive/PLANS/play-now-tab-retroverse.md) —
  PLAY NOW tab design.
- [discover-tab-retroverse.md](../../_archive/PLANS/discover-tab-retroverse.md) —
  DISCOVER tab design. v1 ships 4 data-driven axes; 5 editorial
  axes ship with Phase C6 content packs.
- [collections-tab-retroverse.md](../../_archive/PLANS/collections-tab-retroverse.md)
  — COLLECTIONS tab design.
- [content-packs.md](../../PLANS/content-packs.md) — cross-cutting
  content-pack mechanism (Phase C6 — not yet implemented).

## Where the code lives

Paths reflect the post-`platform/` refactor tree (ARC-1): Retroverse-specific
code now lives under `frontend/src/themes/retroverse/`; shared substrate sits
in `frontend/src/platform/` and `frontend/src/engine/`.

- **Shell + routing:** `frontend/src/themes/retroverse/RetroverseShell.tsx`
  + `frontend/src/themes/retroverse/currentRoute.ts`
- **Per-tab pages:**
  `frontend/src/themes/retroverse/{HomePage,LibraryPage,CollectionsPage,PlayNowPage,DiscoverPage}.tsx`
- **Right-pane components:**
  `frontend/src/themes/retroverse/{GameDetailPanel,SystemInfoPanel}.tsx`
- **Per-system SETTINGS surface:** the lifted sections (Display / Rewind /
  Shaders / Default-core) live in
  `frontend/src/platform/components/perSystemSections.tsx`. The dedicated
  per-system drill-in now lives under `frontend/src/engine/systemsHub/`
  (the Per-System Settings Hub), not a Retroverse route page.
- **Flag accessor:** `frontend/src/platform/lib/retroverseFlag.ts`
- **SETTINGS body components (15 categories):**
  `frontend/src/engine/SettingsSections.tsx`
- **Slice 12 custom collections:**
  `frontend/src/platform/library/customCollections.ts` (store) +
  `frontend/src/platform/components/NewCollectionDialog.tsx` (create/rename).
- **Now-playing chip:** `frontend/src/platform/lib/audio.ts` exports the
  `nowPlaying` accessor; HintBar consumes it.

## Status

See [retroverse-ui-rollout.md §10](../../PLANS/retroverse-ui-rollout.md)
for the canonical status surface.

## Decisions

See [DECISIONS.md](DECISIONS.md) for the load-bearing architecture
decisions made in this feature's history (unified controller
pipeline, custom-collection schema, per-system section sharing,
DISCOVER ship strategy, now-playing source-of-truth).

## Session log

See [SESSION_LOG.md](SESSION_LOG.md) for shipped / almost / next entries.
