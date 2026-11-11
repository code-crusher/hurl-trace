/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
//! Implementation of DPoP (Demonstrating Proof of Possession).
//!
//! See <https://datatracker.ietf.org/doc/html/rfc9449>.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use p256::SecretKey;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer;
use p256::pkcs8::DecodePrivateKey;
use sha2::Digest;
use uuid::Uuid;

use super::error::HttpError;
use super::header::{AUTHORIZATION, DPOP, Header, HeaderVec};

/// The value of the `typ` JOSE header parameter of a DPoP proof.
const DPOP_PROOF_TYPE: &str = "dpop+jwt";
/// The JWS algorithm used to sign a DPoP proof.
const DPOP_PROOF_ALGORITHM: &str = "ES256";

/// A private key used to sign DPoP proofs.
///
/// DPoP proofs are signed with an ECDSA P-256 key (the `ES256` JWS algorithm). The private key is
/// given by the user with the `--dpop-private-key` option, and the matching public key is embedded
/// in every proof (through the `jwk` header parameter) so that the server can verify the signature.
#[derive(Clone, Debug)]
pub struct DpopKey {
    signing_key: SigningKey,
}

impl DpopKey {
    /// Creates a DPoP key from a PEM encoded ECDSA P-256 private key.
    ///
    /// Both PKCS#8 (`-----BEGIN PRIVATE KEY-----`) and SEC1 (`-----BEGIN EC PRIVATE KEY-----`)
    /// encodings are supported.
    pub fn parse(content: &str) -> Result<Self, HttpError> {
        let secret_key = SecretKey::from_pkcs8_pem(content)
            .or_else(|_| SecretKey::from_sec1_pem(content))
            .map_err(|_| {
                let reason = "expected a PEM encoded ECDSA P-256 private key (PKCS#8 or SEC1)";
                HttpError::InvalidDpopKey(reason.to_string())
            })?;
        Ok(DpopKey {
            signing_key: SigningKey::from(secret_key),
        })
    }

    /// Returns a copy of `headers` with a DPoP proof for a request using `method` to `url` (the
    /// final request URL, query string included).
    ///
    /// If `headers` contains a `Bearer` (or already `DPoP`) access token, the proof is bound to
    /// this token with an `ath` claim, and the authorization scheme is changed to `DPoP`, as
    /// required by [RFC 9449 section 7.1](https://datatracker.ietf.org/doc/html/rfc9449#section-7.1).
    pub fn headers(&self, headers: &HeaderVec, method: &str, url: &str) -> HeaderVec {
        let access_token = headers
            .get(AUTHORIZATION)
            .and_then(|header| access_token(&header.value));
        let proof = self.proof(method, url, access_token);

        let mut headers = headers.clone();
        // A `DPoP` header can only be computed by Hurl: the proof is unique to each request and
        // can't be provided by the user.
        headers.retain(|h| !h.name_eq(DPOP));
        if let Some(access_token) = access_token {
            headers.retain(|h| !h.name_eq(AUTHORIZATION));
            headers.push(Header::new(AUTHORIZATION, &format!("DPoP {access_token}")));
        }
        headers.push(Header::new(DPOP, &proof));
        headers
    }

    /// Creates a DPoP proof JWT for a request to `url` (the final request URL, query string
    /// included) using `method`.
    ///
    /// If `access_token` is provided, an `ath` claim is added, binding the proof to this token.
    fn proof(&self, method: &str, url: &str, access_token: Option<&str>) -> String {
        let header = format!(
            r#"{{"typ":"{DPOP_PROOF_TYPE}","alg":"{DPOP_PROOF_ALGORITHM}","jwk":{}}}"#,
            self.jwk()
        );

        // `htu` is the HTTP target URI, without query and fragment parts.
        let htu = strip_query_and_fragment(url);

        let mut claims = format!(
            r#"{{"jti":"{}","htm":"{}","htu":"{}","iat":{}"#,
            Uuid::new_v4(),
            escape_json(method),
            escape_json(htu),
            Utc::now().timestamp()
        );
        if let Some(access_token) = access_token {
            let mut hasher = sha2::Sha256::new();
            hasher.update(access_token.as_bytes());
            let ath = URL_SAFE_NO_PAD.encode(hasher.finalize());
            claims.push_str(&format!(r#","ath":"{ath}""#));
        }
        claims.push('}');

        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header.as_bytes()),
            URL_SAFE_NO_PAD.encode(claims.as_bytes())
        );
        // The signature is encoded as the raw `R || S` concatenation, as required by JWS.
        let signature: p256::ecdsa::Signature = self.signing_key.sign(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        format!("{signing_input}.{signature}")
    }

    /// Returns the public key embedded in this key, as a JWK JSON object.
    fn jwk(&self) -> String {
        let point = self.signing_key.verifying_key().to_sec1_point(false);
        let x = URL_SAFE_NO_PAD.encode(point.x().expect("P-256 public key has a x coordinate"));
        let y = URL_SAFE_NO_PAD.encode(point.y().expect("P-256 public key has a y coordinate"));
        format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#)
    }

    /// Returns `true` if `proof` is a well-formed DPoP proof signed by this key.
    #[cfg(test)]
    fn verify_proof(&self, proof: &str) -> bool {
        use p256::ecdsa::signature::Verifier;

        let parts = proof.split('.').collect::<Vec<_>>();
        if parts.len() != 3 {
            return false;
        }
        let Ok(signature) = URL_SAFE_NO_PAD.decode(parts[2]) else {
            return false;
        };
        let Ok(signature) = p256::ecdsa::Signature::from_slice(&signature) else {
            return false;
        };
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        self.signing_key
            .verifying_key()
            .verify(signing_input.as_bytes(), &signature)
            .is_ok()
    }
}

/// Returns the access token of an `Authorization` header `value` using the `Bearer` or `DPoP`
/// scheme, if any.
fn access_token(value: &str) -> Option<&str> {
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") && !scheme.eq_ignore_ascii_case("dpop") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() { None } else { Some(token) }
}

/// Returns `url` without its query and fragment parts.
fn strip_query_and_fragment(url: &str) -> &str {
    let end = url.find(['?', '#']).unwrap_or(url.len());
    &url[..end]
}

/// Escapes a string `s` to be used in a JSON string value.
fn escape_json(s: &str) -> String {
    let mut output = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if (c as u32) < 0x20 => output.push_str(&format!("\\u{:04x}", c as u32)),
            c => output.push(c),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use p256::ecdsa::SigningKey;
    use p256::elliptic_curve::Generate;
    use sha2::Digest;

    use super::{DpopKey, access_token, strip_query_and_fragment};
    use crate::http::header::{AUTHORIZATION, DPOP, Header, HeaderVec};

    // A PKCS#8 encoded P-256 private key, generated with:
    // $ openssl ecparam -name prime256v1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt
    const PKCS8_KEY: &str = "-----BEGIN PRIVATE KEY-----\n\
                             MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgevZzL1gdAFr88hb2\n\
                             OF/2NxApJCzGCEDdfSp6VQO30hyhRANCAAQRWz+jn65BtOMvdyHKcvjBeBSDZH2r\n\
                             1RTwjmYSi9R/zpBnuQ4EiMnCqfMPWiZqB4QdbAd0E7oH50VpuZ1P087G\n\
                             -----END PRIVATE KEY-----\n";

    // A SEC1 encoded P-256 private key, generated with:
    // $ openssl ecparam -name prime256v1 -genkey -noout
    const SEC1_KEY: &str = "-----BEGIN EC PRIVATE KEY-----\n\
                            MHcCAQEEILKeMKRe2+e/5tVaUiln7iSTsgm8w5xHU0bDRo8M4LC2oAoGCCqGSM49\n\
                            AwEHoUQDQgAEXlOsw0RPLOBh04HLJQBaOlCUO8gf5eQ45yc3SQU9ACX/HIq4HwCp\n\
                            KhLqYLnaN1RVQ1jg0akZNPR5YIyHQ03ctg==\n\
                            -----END EC PRIVATE KEY-----\n";

    fn new_key() -> DpopKey {
        DpopKey {
            signing_key: SigningKey::generate(),
        }
    }

    #[test]
    fn parse_pkcs8_key() {
        let key = DpopKey::parse(PKCS8_KEY).unwrap();
        assert!(key.verify_proof(&key.proof("GET", "https://example.org/token", None)));
    }

    #[test]
    fn parse_sec1_key() {
        let key = DpopKey::parse(SEC1_KEY).unwrap();
        assert!(key.verify_proof(&key.proof("GET", "https://example.org/token", None)));
    }

    #[test]
    fn parse_invalid_key() {
        assert!(DpopKey::parse("not a key").is_err());
        assert!(DpopKey::parse("").is_err());
    }

    #[test]
    fn proof_headers_and_claims() {
        let key = new_key();
        let proof = key.proof("POST", "https://example.org/foo/bar?baz=qux#fragment", None);
        let parts = proof.split('.').collect::<Vec<_>>();
        assert_eq!(parts.len(), 3);

        let header = decode_json(parts[0]);
        assert!(header.contains(r#""typ":"dpop+jwt""#));
        assert!(header.contains(r#""alg":"ES256""#));
        assert!(header.contains(r#""crv":"P-256""#));
        assert!(header.contains(r#""kty":"EC""#));
        assert!(!header.contains(r#""d":"#));

        let claims = decode_json(parts[1]);
        assert!(claims.contains(r#""htm":"POST""#));
        // Query and fragment are not part of `htu`.
        assert!(claims.contains(r#""htu":"https://example.org/foo/bar""#));
        assert!(claims.contains(r#""jti":"#));
        assert!(claims.contains(r#""iat":"#));
        assert!(!claims.contains(r#""ath":"#));

        assert!(key.verify_proof(&proof));
    }

    #[test]
    fn proof_with_access_token() {
        let key = new_key();
        let proof = key.proof("GET", "https://example.org/token", Some("my-token"));
        let claims = decode_json(proof.split('.').nth(1).unwrap());

        // `ath` is the base64url encoded SHA-256 hash of the access token.
        let mut hasher = sha2::Sha256::new();
        hasher.update("my-token".as_bytes());
        let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert!(claims.contains(&format!(r#""ath":"{expected}""#)));
    }

    #[test]
    fn proof_is_unique_per_request() {
        let key = new_key();
        let proof1 = key.proof("GET", "https://example.org/token", None);
        let proof2 = key.proof("GET", "https://example.org/token", None);
        assert_ne!(proof1, proof2);
    }

    #[test]
    fn proof_signed_with_another_key_is_rejected() {
        let key1 = new_key();
        let key2 = new_key();
        assert!(!key2.verify_proof(&key1.proof("GET", "https://example.org/token", None)));
    }

    #[test]
    fn headers_with_bearer_token() {
        let key = new_key();
        let mut headers = HeaderVec::new();
        headers.push(Header::new(AUTHORIZATION, "Bearer my-token"));
        headers.push(Header::new("Accept", "application/json"));

        let headers = key.headers(&headers, "GET", "https://example.org/foo?bar=baz");

        // The `Bearer` scheme is replaced by `DPoP`, the access token is kept.
        assert_eq!(
            headers.get(AUTHORIZATION),
            Some(&Header::new(AUTHORIZATION, "DPoP my-token"))
        );
        // Other headers are kept.
        assert_eq!(
            headers.get("Accept"),
            Some(&Header::new("Accept", "application/json"))
        );
        let proof = headers.get(DPOP).expect("DPoP header").value.clone();
        assert!(key.verify_proof(&proof));
        let claims = decode_json(proof.split('.').nth(1).unwrap());
        assert!(claims.contains(r#""htu":"https://example.org/foo""#));
        assert!(claims.contains(r#""ath":"#));
    }

    #[test]
    fn headers_without_access_token() {
        let key = new_key();
        let mut headers = HeaderVec::new();
        headers.push(Header::new("Accept", "application/json"));

        let headers = key.headers(&headers, "GET", "https://example.org/foo");

        assert_eq!(headers.get(AUTHORIZATION), None);
        assert!(headers.get(DPOP).is_some());
    }

    #[test]
    fn headers_override_user_provided_proof() {
        let key = new_key();
        let mut headers = HeaderVec::new();
        headers.push(Header::new(DPOP, "a-static-proof"));

        let headers = key.headers(&headers, "GET", "https://example.org/foo");

        let proofs = headers.get_all(DPOP);
        assert_eq!(proofs.len(), 1);
        assert_ne!(proofs[0].value, "a-static-proof");
    }

    #[test]
    fn access_token_parts() {
        assert_eq!(access_token("Bearer abc"), Some("abc"));
        assert_eq!(access_token("bearer abc"), Some("abc"));
        assert_eq!(access_token("DPoP abc"), Some("abc"));
        assert_eq!(access_token("Basic abc"), None);
        assert_eq!(access_token("Bearer"), None);
        assert_eq!(access_token("Bearer "), None);
    }

    #[test]
    fn strip_query_and_fragment_parts() {
        assert_eq!(
            strip_query_and_fragment("https://example.org/foo"),
            "https://example.org/foo"
        );
        assert_eq!(
            strip_query_and_fragment("https://example.org/foo?baz=qux"),
            "https://example.org/foo"
        );
        assert_eq!(
            strip_query_and_fragment("https://example.org/foo#fragment"),
            "https://example.org/foo"
        );
        assert_eq!(
            strip_query_and_fragment("https://example.org/foo?baz=qux#fragment"),
            "https://example.org/foo"
        );
    }

    fn decode_json(part: &str) -> String {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(part)
            .unwrap();
        String::from_utf8(bytes).unwrap()
    }
}
