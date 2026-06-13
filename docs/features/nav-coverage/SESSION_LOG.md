# Controller-Nav Coverage — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-13 — Stream queued

- **Shipped:** Nothing in code yet — stream created. Scope + prioritized gap
  table + standard recipe captured in [README.md](README.md), backed by the
  [nav-coverage audit](../controller-identity/NAV_COVERAGE_AUDIT_2026-06-12.md)
  run during the controller-identity arc. Queued in NEXT.md HIGH band.
- **Almost:** n/a — paperwork only.
- **Next:** **Slice 1 — Settings category bodies.** Wire row-by-row controller
  nav into the per-category Settings forms (`SettingsSections.tsx`,
  `PerSystemSettingsBody.tsx`, `MetadataSettingsBody.tsx`) — the sidebar already
  navigates; the bodies don't. Use `useDomQueryFocusGroup` with a row selector;
  verify with the controller test window (Settings → Controllers) + a real pad.
