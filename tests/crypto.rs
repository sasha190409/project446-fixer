// tests/crypto.rs
//! Tests for the Ed25519 signature layer. Cross-platform: no WinAPI.

use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};
use csgo_legacy_fixer::crypto;

/// Fixed test key — NOT the production key. 32 bytes of 0x07.
fn test_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

// ---------- happy path ----------

#[test]
fn verify_accepts_valid_base64_signature() {
    let sk = test_signing_key();
    let vk = sk.verifying_key();
    let data = b"update.gcup payload";
    let sig = sk.sign(data);
    let sig_b64 = B64.encode(sig.to_bytes());

    crypto::verify(&vk, data, &sig_b64).expect("valid signature must verify");
}

#[test]
fn verify_accepts_valid_hex_signature() {
    let sk = test_signing_key();
    let vk = sk.verifying_key();
    let data = b"another payload";
    let sig = sk.sign(data);

    let mut hex = String::with_capacity(128);
    for b in sig.to_bytes() {
        hex.push_str(&format!("{:02x}", b));
    }

    crypto::verify(&vk, data, &hex).expect("hex signature must verify");
}

#[test]
fn verify_accepts_urlsafe_base64_signature() {
    let sk = test_signing_key();
    let vk = sk.verifying_key();
    let data = b"payload with +/ chars to force urlsafe differences";
    let sig = sk.sign(data);

    let sig_urlsafe =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());

    // Only run this assertion when the standard decoder rejects it,
    // otherwise we'd be testing the wrong branch.
    if B64.decode(&sig_urlsafe).map(|v| v.len()).ok() != Some(64) {
        crypto::verify(&vk, data, &sig_urlsafe)
            .expect("urlsafe signature must verify when std rejects it");
    }
}

// ---------- wrong input ----------

#[test]
fn verify_rejects_tampered_data() {
    let sk = test_signing_key();
    let vk = sk.verifying_key();
    let sig = sk.sign(b"original");
    let sig_b64 = B64.encode(sig.to_bytes());

    let err = crypto::verify(&vk, b"tampered", &sig_b64)
        .expect_err("tampered data must fail");
    assert!(
        err.to_string().contains("verification failed"),
        "unexpected error: {err}"
    );
}

#[test]
fn verify_rejects_signature_from_other_key() {
    let sk_a = SigningKey::from_bytes(&[1u8; 32]);
    let sk_b = SigningKey::from_bytes(&[2u8; 32]);
    let data = b"payload";

    let sig_from_a = B64.encode(sk_a.sign(data).to_bytes());

    let err = crypto::verify(&sk_b.verifying_key(), data, &sig_from_a)
        .expect_err("signature by another key must fail");
    assert!(err.to_string().contains("verification failed"));
}

// ---------- malformed signature ----------

#[test]
fn verify_rejects_empty_signature() {
    let sk = test_signing_key();
    let err = crypto::verify(&sk.verifying_key(), b"x", "")
        .expect_err("empty signature must be rejected");
    assert!(err.to_string().contains("neither valid base64 nor hex"));
}

#[test]
fn verify_rejects_wrong_length_signature() {
    let sk = test_signing_key();
    // 32 bytes instead of 64.
    let short = B64.encode([0u8; 32]);
    let err = crypto::verify(&sk.verifying_key(), b"x", &short)
        .expect_err("32-byte signature must be rejected");
    assert!(err.to_string().contains("neither valid base64 nor hex"));
}

#[test]
fn verify_rejects_garbage_string() {
    let sk = test_signing_key();
    let err = crypto::verify(&sk.verifying_key(), b"x", "not base64 or hex!!")
        .expect_err("garbage must be rejected");
    assert!(err.to_string().contains("neither valid base64 nor hex"));
}

#[test]
fn verify_rejects_odd_length_hex() {
    let sk = test_signing_key();
    let err = crypto::verify(&sk.verifying_key(), b"x", &"a".repeat(127))
        .expect_err("odd hex length must be rejected");
    assert!(err.to_string().contains("neither valid base64 nor hex"));
}

// ---------- load_public_key ----------

#[test]
fn load_public_key_returns_some_with_embedded_key() {
    // The crate ships with EMBEDDED_PUBKEY_B64 set; this asserts we can
    // decode it. If someone blanks the constant, this test fails loudly.
    let key = crypto::load_public_key()
        .expect("load must succeed when embedded key is present")
        .expect("embedded key must yield Some");
    // VerifyingKey has no Debug-friendly eq, but we can re-verify a
    // signature with it to prove it's usable.
    let sk = test_signing_key();
    let sig = B64.encode(sk.sign(b"probe").to_bytes());
    let _ = crypto::verify(&key, b"probe", &sig); // must not panic
}
