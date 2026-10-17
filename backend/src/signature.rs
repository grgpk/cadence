//! Signed requests between Cadence and another server: Stripe's scheme,
//! which Cadence also signs its own webhooks with.
//!
//! The header is `t=<unix seconds>,v1=<hex HMAC-SHA256 of "t.body">`, possibly
//! with several `v1` values while a secret is being rolled.

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use thiserror::Error;

/// How old a signed request may be, against replays.
const TOLERANCE_SECONDS: u64 = 300;
/// The most `v1` values a header may carry. Stripe sends one, or a few while
/// a secret is being rolled.
const SIGNATURES_MAX: usize = 8;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("the signature does not match")]
pub struct BadSignature;

fn mac(secret: &str, timestamp: i64, payload: &[u8]) -> Option<Hmac<Sha256>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    Some(mac)
}

/// The header for `payload` signed with `secret` at `at`.
pub fn sign(secret: &str, payload: &[u8], at: DateTime<Utc>) -> String {
    let timestamp = at.timestamp();
    let signature = mac(secret, timestamp, payload)
        .map(|mac| hex::encode(mac.finalize().into_bytes()))
        .unwrap_or_default();
    format!("t={timestamp},v1={signature}")
}

/// Checks a signature header against the raw body.
pub fn verify(
    secret: &str,
    payload: &[u8],
    header: &str,
    now: DateTime<Utc>,
) -> Result<(), BadSignature> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signatures.push(value),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or(BadSignature)?;
    // `abs_diff` cannot overflow, whatever timestamp the header claims.
    if signatures.len() > SIGNATURES_MAX || now.timestamp().abs_diff(timestamp) > TOLERANCE_SECONDS
    {
        return Err(BadSignature);
    }
    // The body is hashed once; each candidate is compared to that.
    let mac = mac(secret, timestamp, payload).ok_or(BadSignature)?;
    let matches = |signature: &&str| {
        hex::decode(signature).is_ok_and(|expected| mac.clone().verify_slice(&expected).is_ok())
    };
    if signatures.iter().any(matches) {
        Ok(())
    } else {
        Err(BadSignature)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    const SECRET: &str = "whsec_test_secret";
    const PAYLOAD: &[u8] = br#"{"id":"evt_1"}"#;
    /// HMAC-SHA256 of `1700000000.{"id":"evt_1"}`, computed outside Rust.
    const SIGNATURE: &str = "248a374f50f943a28b0f6ab50faf9a7e7e29b710fa26df9fb1618b9bf8ea9c9a";

    fn at(seconds: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(seconds, 0).single().unwrap_or_default()
    }

    #[test]
    fn a_known_signature_verifies() {
        let header = format!("t=1700000000,v1={SIGNATURE}");
        assert_eq!(verify(SECRET, PAYLOAD, &header, at(1_700_000_010)), Ok(()));
        assert_eq!(sign(SECRET, PAYLOAD, at(1_700_000_000)), header);
    }

    #[test]
    fn any_v1_may_match_while_secrets_roll() {
        let header = format!("t=1700000000,v1=00ff,v1={SIGNATURE},v0=abc");
        assert_eq!(verify(SECRET, PAYLOAD, &header, at(1_700_000_000)), Ok(()));
    }

    #[test]
    fn a_changed_body_an_old_request_or_no_timestamp_is_refused() {
        let header = format!("t=1700000000,v1={SIGNATURE}");
        let now = at(1_700_000_000);
        assert_eq!(verify(SECRET, b"{}", &header, now), Err(BadSignature));
        assert_eq!(
            verify(SECRET, PAYLOAD, &header, at(1_700_000_301)),
            Err(BadSignature)
        );
        assert_eq!(
            verify(SECRET, PAYLOAD, &format!("v1={SIGNATURE}"), now),
            Err(BadSignature)
        );
        assert_eq!(
            verify("whsec_other", PAYLOAD, &header, now),
            Err(BadSignature)
        );
    }

    #[test]
    fn too_many_signatures_or_an_extreme_timestamp_is_refused() {
        let now = at(1_700_000_000);
        let padding = "v1=00,".repeat(SIGNATURES_MAX);
        let header = format!("t=1700000000,{padding}v1={SIGNATURE}");
        assert_eq!(verify(SECRET, PAYLOAD, &header, now), Err(BadSignature));
        for timestamp in [i64::MIN, i64::MAX] {
            let header = format!("t={timestamp},v1={SIGNATURE}");
            assert_eq!(verify(SECRET, PAYLOAD, &header, now), Err(BadSignature));
        }
    }
}
