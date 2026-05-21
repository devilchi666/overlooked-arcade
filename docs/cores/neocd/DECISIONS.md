# neocd Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — NeoCD as the Neo Geo CD core

**Decision:** Default to `neocd_libretro.dll` (NeoCD Redux). FBNeo can
also drive Neo Geo CD via its arcade core but NeoCD is the dedicated
libretro path with better CD-specific compatibility (BIOS handling,
CDDA streaming, save data).

---

## 2026-05-20 — neocd separate SystemId, shared controller with neogeo

**Decision:** Neo Geo CD is `neocd`, distinct slug from cart `neogeo`.
Different cores (FBNeo vs NeoCD), different BIOSes, different load
paths (Bytes vs Path-based + CD BIOS pre-check). Shares the 4-button
arcade controller via the `"neogeo" | "neocd" => ...` dispatch arms
in bindings.rs (same precedent PCE-CD / TG-16 set, segacd / genesis
set, sega32x / genesis set).

---

## 2026-05-20 — Muted SNK gold 50° theme (family-cousin to cart neogeo)

**Decision:** `[data-system="neocd"]` ships `oklch(0.55 0.18 50)` —
muted gold sitting in the tight gap between sega32x neon orange (42°)
and TG-16 warm orange (55°). The L=0.55 + C=0.18 profile reads as
"Neo Geo CD gold-bronze" rather than vivid neon orange or saturated
warm orange.

**Why:** Period-correct to Neo Geo CD-Z / CD-T marketing (silver +
gold accents on black hardware). Stays in the warm zone alongside
cart neogeo (deepest red 18°) to preserve the SNK arcade family
visual relationship — red + gold = canonical Neo Geo branding.

Hue 50° lives in a tight 13° gap between sega32x 42° and TG-16 55°
but the L/C profile separates all three:
- sega32x: 42°/L=0.68/C=0.22 — neon bright
- **neocd: 50°/L=0.55/C=0.18 — muted gold-bronze**
- TG-16: 55°/L=0.74/C=0.18 — warm orange

---

## 2026-05-20 — Three canonical BIOS SHA-1s, OkUnknownHash fallback

**Decision:** `NEOCD_BIOS_KNOWN_HASHES` ships with three entries: CDZ
top-loader (`neocd_z.rom`), CD front-loader (`neocd_t.rom`), and the
front-loader alternate naming (`neocd_f.rom`). The top-loader is the
most-commonly-tested dump.

**Why:** The two Neo Geo CD hardware models (CDZ top-loader v1, CD/CDT
front-loader v2) shipped distinct BIOS chips that are functionally
interchangeable for game launch. Operators with either dump should
hit OkCanonical; Unibios CD variants get OkUnknownHash + warn-toast.
