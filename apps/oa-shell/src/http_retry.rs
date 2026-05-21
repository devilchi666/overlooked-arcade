//! Tiny HTTP-fetch helper with transient/permanent error classification
//! and one retry on transient failures.
//!
//! Used by the libretro-thumbnails / libretro-database / metadat
//! fetchers. Pre-fix each treated any non-success status as fatal for
//! the request — a transient 503 from GitHub's CDN failed the whole
//! sync for that file. Now: 4xx (not 404) is permanent, 404 is the
//! explicit "this resource genuinely doesn't exist" semantic, 5xx
//! and network errors get one retry after a 1s backoff.
//!
//! Why one retry, not exponential or unbounded:
//! - libretro-thumbnails / libretro-database are served from GitHub
//!   raw + GitHub's API, both of which have very high availability.
//!   Most transient errors clear on the very next attempt.
//! - Sync calls run in `buffer_unordered(8)` loops with 1000s of
//!   per-ROM requests. Aggressive retry policies multiply the load
//!   on GitHub if the outage actually persists; better to fail fast
//!   on the second attempt and let the operator try again later.
//! - A fixed 1s sleep is enough for a microservice hiccup but short
//!   enough that the operator doesn't feel the delay if it's the only
//!   request in flight.

use std::time::Duration;

/// Outcome of one HTTP attempt — drives the retry decision.
#[derive(Debug)]
enum Outcome {
    /// HTTP 404 — file genuinely doesn't exist. Don't retry.
    NotFound,
    /// 2xx — return the response.
    Got(reqwest::Response),
    /// 4xx (not 404) — bad request shape. Don't retry.
    Permanent(String),
    /// 5xx or network error — worth one retry.
    Transient(String),
}

async fn try_once(client: &reqwest::Client, url: &str, ua: &str) -> Outcome {
    match client
        .get(url)
        .header("User-Agent", ua)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status == reqwest::StatusCode::NOT_FOUND {
                Outcome::NotFound
            } else if status.is_success() {
                Outcome::Got(resp)
            } else if status.is_client_error() {
                Outcome::Permanent(format!("status {status}"))
            } else {
                // 5xx range (or weird 3xx that we don't follow)
                Outcome::Transient(format!("status {status}"))
            }
        }
        // Network-level errors: DNS, refused connection, timeout, TLS handshake, etc.
        // All treated as transient — one retry handles GitHub CDN hiccups + the
        // momentary blip of a wifi roam.
        Err(e) => Outcome::Transient(format!("network: {e}")),
    }
}

/// Fetch with one retry on transient failures. Returns:
/// - `Ok(Some(response))` — fetched successfully
/// - `Ok(None)` — 404 (resource genuinely absent)
/// - `Err(message)` — permanent failure or transient that persisted after retry
pub async fn get_with_retry(
    client: &reqwest::Client,
    url: &str,
    ua: &str,
) -> Result<Option<reqwest::Response>, String> {
    match try_once(client, url, ua).await {
        Outcome::Got(resp) => return Ok(Some(resp)),
        Outcome::NotFound => return Ok(None),
        Outcome::Permanent(msg) => return Err(format!("{url}: {msg}")),
        Outcome::Transient(msg) => {
            log::warn!("oa-shell: HTTP {url} transient ({msg}); retrying in 1s");
        }
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    match try_once(client, url, ua).await {
        Outcome::Got(resp) => Ok(Some(resp)),
        Outcome::NotFound => Ok(None),
        Outcome::Permanent(msg) => Err(format!("{url}: {msg} (after retry)")),
        Outcome::Transient(msg) => Err(format!("{url}: {msg} (after retry)")),
    }
}

/// Same as `get_with_retry` but reads the body as text on success.
/// Convenience for the .dat fetchers that always treat the body as
/// UTF-8 text.
pub async fn get_text_with_retry(
    client: &reqwest::Client,
    url: &str,
    ua: &str,
) -> Result<Option<String>, String> {
    let Some(resp) = get_with_retry(client, url, ua).await? else {
        return Ok(None);
    };
    resp.text()
        .await
        .map(Some)
        .map_err(|e| format!("read body {url}: {e}"))
}
