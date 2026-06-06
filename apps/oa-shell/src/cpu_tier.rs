//! CPU detection + tier bucketing — Phase 2 of the Guided Setup arc.
//!
//! Detects the host CPU's brand + physical-core count + base/current
//! clock via `sysinfo`, buckets into one of three power tiers
//! (High / Mid / Low), and caches the result to `<appDataDir>/cpu-tier.json`
//! so the detection runs once at first launch and subsequent boots
//! short-circuit to the cached value.
//!
//! The tier drives the per-system core recommendation table in
//! `core_installer.rs` (`recommended_core_for_tier`) — picks
//! `beetle_psx_hw` on High vs `pcsx_rearmed` on Low for PSX, `bsnes`
//! on High vs `snes9x` on Mid for SNES, etc.
//!
//! ## Decision: heuristic is source of truth
//!
//! Locked 2026-06-06 per `docs/PLANS/guided-setup.md` §7
//! "Decision locked 2026-06-06". The static heuristic ships as the
//! Phase 2 recommendation engine — not as a placeholder until
//! benchmarks land. Rationale: most operators never run a benchmark,
//! so the heuristic has to stand on its own for the common case.
//! Treating it as a placeholder would breed sloppy thresholds.
//!
//! Known limitations the heuristic cannot solve (operator-override
//! escape hatch is the documented mitigation):
//! - Steam Deck (Zen 2 mobile rates low on cores+clock but iGPU is
//!   strong → underrated against real emulation capability).
//! - Old high-end Xeon workstations (rate Mid/High by core count but
//!   lose to modern Ryzen 5600G at every real workload).
//! - Thermal-throttled thin-and-light laptops (boost clock looks
//!   good, sustained performance is half the rating).
//! - GPU-bound cores (Beetle PSX HW, Flycast, PCSX2 don't fit the
//!   CPU-centric heuristic well).
//!
//! Possible Phase 2B (not committed): synthetic-stress benchmark
//! mode that overrides the heuristic per-host. Designed to be
//! architecturally additive — `recommended_core_for_system` would
//! grow an optional `benchmark_results` parameter that falls back to
//! the tier-from-heuristic when missing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sysinfo::System;

/// One of three power tiers a host CPU falls into. Drives the
/// per-system core recommendation table in `core_installer.rs`.
///
/// Also persists as `LibraryPrefs.cpu_tier_override` when the operator
/// has picked a specific tier in Settings → Performance — that
/// override wins regardless of detected hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuTier {
    /// 6+ physical cores at ≥3.0 GHz. Modern desktop (Ryzen
    /// 5600+ / Intel 10th-gen+) target.
    High,
    /// 4+ physical cores at ≥2.5 GHz. Modern quad-core laptops + older
    /// 6-core desktops.
    Mid,
    /// Everything else — older laptops, low-power chips, mobile.
    Low,
}

/// CPU information snapshot captured at detection time. Mirrors what
/// `sysinfo` returns plus a normalized `base_clock_ghz` derived either
/// from the brand string ("@ 3.6 GHz" suffix that Intel/AMD include in
/// most brand reports) or from the current frequency at detection
/// time as a fallback.
///
/// Serializable for both the on-disk cache + the Tauri command
/// response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    /// Vendor's CPU brand string (e.g. "AMD Ryzen 5 5600X 6-Core
    /// Processor", "Intel(R) Core(TM) i7-12700K @ 3.6 GHz").
    pub brand: String,
    /// Physical core count. We deliberately ignore SMT/Hyper-Threading
    /// — for the High/Mid threshold, hyperthreads don't add much
    /// emulation throughput.
    pub cores_physical: u32,
    /// Base clock in GHz. Best-effort: parsed from `brand` when the
    /// vendor included it ("@ 3.6 GHz" suffix); falls back to the
    /// current frequency at detection time when parsing fails.
    pub base_clock_ghz: f32,
}

/// Where the resolved tier came from. Frontend's Settings → Performance
/// card renders a small chip ("Detected" / "Cached" / "Override")
/// explaining the provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierSource {
    /// Computed this session via `sysinfo` + `bucket_into_tier`.
    Detected,
    /// Read from the cached `<appDataDir>/cpu-tier.json` file.
    Cached,
    /// `LibraryPrefs.cpu_tier_override = Some(...)` is set; that
    /// value wins regardless of detected hardware.
    Override,
}

/// Full snapshot returned by the `detect_cpu_tier` Tauri command.
/// The frontend uses `info` to render the read-only hardware display
/// (brand + cores + clock) and `tier` + `source` to render the tier
/// chip + provenance hint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuTierSnapshot {
    pub tier: CpuTier,
    pub source: TierSource,
    pub info: CpuInfo,
}

/// Bucket detected CPU info into a power tier per the
/// `docs/PLANS/guided-setup.md` §7 thresholds.
///
/// Thresholds:
/// - **High:** 6+ physical cores AND `base_clock_ghz >= 3.0`
/// - **Mid:**  4+ physical cores AND `base_clock_ghz >= 2.5`
/// - **Low:**  everything else
///
/// Brand-string parsing for vendor generations ("Intel 10th gen+",
/// "AMD Ryzen 3000+") was considered and dropped for v1 — string
/// formats vary across vendors / locales / silicon revisions, and
/// (cores, clock) captures most of the signal an emulator launcher
/// needs. Operators with hardware on the wrong side of a boundary
/// (Steam Deck, old Xeons) override via Settings → Performance.
pub fn bucket_into_tier(info: &CpuInfo) -> CpuTier {
    if info.cores_physical >= 6 && info.base_clock_ghz >= 3.0 {
        CpuTier::High
    } else if info.cores_physical >= 4 && info.base_clock_ghz >= 2.5 {
        CpuTier::Mid
    } else {
        CpuTier::Low
    }
}

/// Detect CPU info via `sysinfo`. Reads physical core count, brand
/// string, and current frequency. Refreshes the CPU sample once before
/// reading — sysinfo's frequency reading is a snapshot of right now,
/// not a baseline.
///
/// The current-frequency reading is at detection time — for our
/// purposes (computing a static tier bucket once at first launch)
/// it's good enough. Aggressive power-managed CPUs idling at 800 MHz
/// at detection time will misclassify, but these are the same
/// hardware classes the plan doc's "Known limitations" subsection
/// already calls out for the operator-override path.
pub fn detect_cpu_info() -> CpuInfo {
    let mut sys = System::new();
    sys.refresh_cpu_all();

    let cpus = sys.cpus();
    let brand = cpus
        .first()
        .map(|c| c.brand().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown CPU".into());
    let cores_physical = sys.physical_core_count().unwrap_or(cpus.len()) as u32;

    // Prefer parsing the brand string for a base-clock declaration
    // (Intel/AMD brands carry "@ N.N GHz" suffixes on most SKUs);
    // fall back to the current frequency reading.
    let base_clock_ghz = parse_clock_from_brand(&brand).unwrap_or_else(|| {
        // sysinfo's `Cpu::frequency()` returns MHz; convert to GHz.
        let mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);
        mhz as f32 / 1000.0
    });

    CpuInfo {
        brand,
        cores_physical,
        base_clock_ghz,
    }
}

/// Parse a clock declaration ("@ 3.6 GHz", "@3.40GHz") out of a CPU
/// brand string. Returns `None` when no match.
///
/// Pattern: locate `@`, then scan to the first occurrence of the
/// case-insensitive "GHz" unit, parse the trimmed substring between
/// them as `f32`.
///
/// Handles:
/// - `"AMD Ryzen 5 5600X @ 3.7GHz"` → 3.7
/// - `"Intel(R) Core(TM) i7-12700K @ 3.6 GHz"` → 3.6
/// - `"Intel(R) Xeon(R) E5-2680 v2 @ 2.80GHz"` → 2.80
/// - `"Apple M2 Pro"` → None (no `@`)
/// - `"Cortex-A78"` → None (no `@`)
fn parse_clock_from_brand(brand: &str) -> Option<f32> {
    let at_idx = brand.find('@')?;
    let after_at = &brand[at_idx + 1..];
    let lower_after = after_at.to_lowercase();
    let ghz_idx = lower_after.find("ghz")?;
    let num_str = after_at[..ghz_idx].trim();
    num_str.parse::<f32>().ok()
}

/// On-disk cache shape — `<appDataDir>/cpu-tier.json`. Stored so the
/// detection runs once at install/first-launch and subsequent boots
/// short-circuit. Cache invalidates only when the brand string
/// changes (operator swapped CPU or moved their install across
/// hardware).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedTier {
    info: CpuInfo,
    tier: CpuTier,
}

/// Path to the cache file under `<appDataDir>/cpu-tier.json`.
pub fn cpu_tier_cache_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cpu-tier.json")
}

/// Detect or load the CPU tier, honoring the operator override when
/// set. Returns the full snapshot so the Tauri command surface can
/// render provenance.
///
/// Resolution chain:
/// 1. If `override_tier` is `Some`, returns it unconditionally with
///    `source = Override` and freshly-detected `info`. The override
///    takes priority — operators with miscategorized hardware
///    (Steam Deck, old Xeon, etc.) get exactly what they asked for.
/// 2. Otherwise, try reading + parsing `<appDataDir>/cpu-tier.json`.
///    If present AND the cached brand string still matches the
///    detected one, return with `source = Cached`. Brand mismatch
///    triggers a re-detection log line + invalidates the cache.
/// 3. Otherwise, freshly bucket the detected info, write the result
///    to the cache, return with `source = Detected`.
///
/// The cache file is best-effort — read/write failures log a warning
/// and proceed without caching. Operator-visible: subsequent launches
/// run the full detection again, ~50 ms wasted per launch.
pub fn detect_or_load(app_data_dir: &Path, override_tier: Option<CpuTier>) -> CpuTierSnapshot {
    let detected_info = detect_cpu_info();

    if let Some(tier) = override_tier {
        return CpuTierSnapshot {
            tier,
            source: TierSource::Override,
            info: detected_info,
        };
    }

    let cache_path = cpu_tier_cache_path(app_data_dir);
    if let Ok(raw) = std::fs::read_to_string(&cache_path) {
        if let Ok(cached) = serde_json::from_str::<CachedTier>(&raw) {
            if cached.info.brand == detected_info.brand {
                return CpuTierSnapshot {
                    tier: cached.tier,
                    source: TierSource::Cached,
                    info: cached.info,
                };
            }
            log::info!(
                "oa-shell: cpu_tier cache brand mismatch ({} → {}); re-detecting",
                cached.info.brand,
                detected_info.brand,
            );
        }
    }

    let tier = bucket_into_tier(&detected_info);
    let cached = CachedTier {
        info: detected_info.clone(),
        tier,
    };
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(&cached) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&cache_path, json) {
                log::warn!("oa-shell: cpu_tier cache write failed: {e}");
            }
        }
        Err(e) => log::warn!("oa-shell: cpu_tier serialize failed: {e}"),
    }

    CpuTierSnapshot {
        tier,
        source: TierSource::Detected,
        info: detected_info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(cores: u32, ghz: f32) -> CpuInfo {
        CpuInfo {
            brand: "Test CPU".into(),
            cores_physical: cores,
            base_clock_ghz: ghz,
        }
    }

    // ---- bucket_into_tier -------------------------------------------

    #[test]
    fn bucket_high_tier_for_modern_desktop() {
        // Ryzen 5 5600X / i5-12400 class: 6 cores at 3.5+ GHz.
        assert_eq!(bucket_into_tier(&info(6, 3.6)), CpuTier::High);
        // Higher-tier (Ryzen 7700X / i7-13700K).
        assert_eq!(bucket_into_tier(&info(12, 4.5)), CpuTier::High);
        // Boundary: exactly 6 cores at exactly 3.0 GHz.
        assert_eq!(bucket_into_tier(&info(6, 3.0)), CpuTier::High);
    }

    #[test]
    fn bucket_mid_tier_for_modern_quad_or_older_six() {
        // 4-core mobile chip at modest clock.
        assert_eq!(bucket_into_tier(&info(4, 2.8)), CpuTier::Mid);
        // 4-core with High-tier clock but cores below High threshold.
        assert_eq!(bucket_into_tier(&info(4, 3.5)), CpuTier::Mid);
        // 6+ cores at slower clock (old Ryzen 1st-gen / Xeon E5 era).
        assert_eq!(bucket_into_tier(&info(8, 2.8)), CpuTier::Mid);
        // Boundary: exactly 4 cores at exactly 2.5 GHz.
        assert_eq!(bucket_into_tier(&info(4, 2.5)), CpuTier::Mid);
    }

    #[test]
    fn bucket_low_tier_for_older_or_mobile() {
        // 2-core older laptop.
        assert_eq!(bucket_into_tier(&info(2, 2.0)), CpuTier::Low);
        // 4-core very low clock (Atom / Celeron N).
        assert_eq!(bucket_into_tier(&info(4, 1.8)), CpuTier::Low);
        // 4-core at 2.4 GHz — just below Mid threshold.
        assert_eq!(bucket_into_tier(&info(4, 2.4)), CpuTier::Low);
    }

    #[test]
    fn bucket_documented_limitation_steam_deck() {
        // Steam Deck (4-core Zen 2 mobile at 2.4 GHz base, 3.5 GHz
        // boost) — documented limitation per the plan doc's
        // "Known limitations" subsection. At base clock it classifies
        // Low; at boost clock it climbs to Mid. iGPU contribution
        // isn't captured at all. Operator override is the escape hatch.
        let deck_base = info(4, 2.4);
        assert_eq!(bucket_into_tier(&deck_base), CpuTier::Low);
        let deck_boost = info(4, 3.5);
        assert_eq!(bucket_into_tier(&deck_boost), CpuTier::Mid);
    }

    // ---- parse_clock_from_brand -------------------------------------

    #[test]
    fn parse_clock_from_brand_at_suffix_variants() {
        // No space between number and "GHz".
        assert_eq!(parse_clock_from_brand("AMD Ryzen 5 5600X @ 3.7GHz"), Some(3.7));
        // Spaced unit ("@ 3.6 GHz").
        assert_eq!(
            parse_clock_from_brand("Intel(R) Core(TM) i7-12700K @ 3.6 GHz"),
            Some(3.6),
        );
        // Trailing-decimal-zero precision ("2.80GHz").
        assert_eq!(
            parse_clock_from_brand("Intel(R) Xeon(R) E5-2680 v2 @ 2.80GHz"),
            Some(2.80),
        );
        // Mixed-case unit doesn't matter.
        assert_eq!(parse_clock_from_brand("Test @ 1.5 Ghz"), Some(1.5));
    }

    #[test]
    fn parse_clock_from_brand_no_at_suffix_returns_none() {
        // Apple Silicon brand strings don't include the @ N.N GHz
        // suffix at all. Falls back to current-frequency reading.
        assert_eq!(parse_clock_from_brand("Apple M2 Pro"), None);
        // ARM cortex strings.
        assert_eq!(parse_clock_from_brand("Cortex-A78"), None);
        // Empty string.
        assert_eq!(parse_clock_from_brand(""), None);
    }

    #[test]
    fn parse_clock_from_brand_malformed_returns_none() {
        // `@` without a parseable number.
        assert_eq!(parse_clock_from_brand("Test @ unknown GHz"), None);
        // `@` without a `GHz` unit.
        assert_eq!(parse_clock_from_brand("Test @ 3.6 something"), None);
    }
}
