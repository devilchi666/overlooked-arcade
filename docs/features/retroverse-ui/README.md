# Retroverse UI

Experimental top-toolbar IA replacing the legacy sidebar-driven Shell.
Flag-gated behind `Settings → Display → Experimental → Retroverse UI`.

## Design + planning

All design docs live under `docs/PLANS/`:

- **[retroverse-ui-rollout.md](../../PLANS/retroverse-ui-rollout.md)** —
  rollout phase plan + the live "Status surface" + "Remaining work"
  list in §10. Start here.
- [settings-tab-retroverse.md](../../PLANS/settings-tab-retroverse.md) —
  SETTINGS tab design.
- [play-now-tab-retroverse.md](../../PLANS/play-now-tab-retroverse.md) —
  PLAY NOW tab design.
- [discover-tab-retroverse.md](../../PLANS/discover-tab-retroverse.md) —
  DISCOVER tab design (not yet implemented).
- [collections-tab-retroverse.md](../../PLANS/collections-tab-retroverse.md)
  — COLLECTIONS tab design.
- [content-packs.md](../../PLANS/content-packs.md) — cross-cutting
  content-pack mechanism (Phase C6 — not yet implemented).

## Where the code lives

- **Shell + routing:** `frontend/src/layout/retroverse/RetroverseShell.tsx`
  + `frontend/src/routing/currentRoute.ts`
- **Per-tab pages:**
  `frontend/src/routes/retroverse/{HomePage,LibraryPage,CollectionsPage,PlayNowPage,SettingsPage,StubPage}.tsx`
- **Right-pane components:**
  `frontend/src/routes/retroverse/{GameDetailPanel,SystemInfoPanel}.tsx`
- **Stub data driving HOME's system info:**
  `frontend/src/routes/retroverse/systemMetadataStubs.ts`
- **Context for shared state + callbacks:**
  `frontend/src/routes/retroverse/context.tsx`
- **Flag accessor:** `frontend/src/lib/retroverseFlag.ts`
- **SETTINGS body components (12 of 14 categories):**
  `frontend/src/components/SettingsSections.tsx`

## Status

See [retroverse-ui-rollout.md §10](../../PLANS/retroverse-ui-rollout.md)
for the canonical status surface.

## Session log

See [SESSION_LOG.md](SESSION_LOG.md) for shipped / almost / next entries.
