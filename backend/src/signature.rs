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
const TOLERANCE_SECONDS: i64 = 300;

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
    if (now.timestamp() - timestamp).abs() > TOLERANCE_SECONDS {
        return Err(BadSignature);
    }
    let matches = |signature: &&str| {
        let (Ok(expected), Some(mac)) = (hex::decode(signature), mac(secret, timestamp, payload))
        else {
            return false;
        };
        mac.verify_slice(&expected).is_ok()
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
}
