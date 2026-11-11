import base64
import hashlib
import json
import time

from app import app
from flask import request

# Minimal P-256 / ES256 verification, implemented here to avoid adding a Python dependency
# to the integration test suite.
P = 0xFFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFF
A = P - 3
N = 0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551
GX = 0x6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296
GY = 0x4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5
G = (GX, GY)


def point_add(p, q):
    if p is None:
        return q
    if q is None:
        return p
    x1, y1 = p
    x2, y2 = q
    if x1 == x2 and (y1 + y2) % P == 0:
        return None
    if p == q:
        slope = (3 * x1 * x1 + A) * pow(2 * y1, -1, P) % P
    else:
        slope = (y2 - y1) * pow(x2 - x1, -1, P) % P
    x3 = (slope * slope - x1 - x2) % P
    return x3, (slope * (x1 - x3) - y1) % P


def point_mul(k, p):
    result = None
    while k:
        if k & 1:
            result = point_add(result, p)
        p = point_add(p, p)
        k >>= 1
    return result


def is_on_curve(point):
    x, y = point
    return (
        y * y
        - (
            x * x * x
            + A * x
            + 0x5AC635D8AA3A93E7B3EBBD55769886BC651D06B0CC53B0F63BCE3C3E27D2604B
        )
    ) % P == 0


def verify_signature(public_key, message, signature):
    r = int.from_bytes(signature[:32], "big")
    s = int.from_bytes(signature[32:], "big")
    if not (1 <= r < N and 1 <= s < N):
        return False
    w = pow(s, -1, N)
    digest = hashlib.sha256(message).digest()
    e = int.from_bytes(digest, "big")
    point = point_add(point_mul(e * w % N, G), point_mul(r * w % N, public_key))
    return point is not None and point[0] % N == r


def decode_base64url(value):
    return base64.urlsafe_b64decode(value + "=" * (-len(value) % 4))


def verify_proof(proof, method, url, access_token=None):
    """Verifies a DPoP proof as specified by RFC 9449, returns the `jwk` public key."""
    header_b64, payload_b64, signature_b64 = proof.split(".")
    header = json.loads(decode_base64url(header_b64))
    payload = json.loads(decode_base64url(payload_b64))
    signature = decode_base64url(signature_b64)

    assert header["typ"] == "dpop+jwt", header
    assert header["alg"] == "ES256", header

    jwk = header["jwk"]
    assert jwk["kty"] == "EC", jwk
    assert jwk["crv"] == "P-256", jwk
    # The private key must never be part of the proof.
    assert "d" not in jwk, jwk
    public_key = (
        int.from_bytes(decode_base64url(jwk["x"]), "big"),
        int.from_bytes(decode_base64url(jwk["y"]), "big"),
    )
    assert is_on_curve(public_key), jwk

    signing_input = f"{header_b64}.{payload_b64}".encode()
    assert verify_signature(public_key, signing_input, signature), "invalid signature"

    assert payload["htm"] == method, payload
    assert payload["htu"] == url, payload
    assert payload["jti"], payload
    assert abs(payload["iat"] - int(time.time())) < 60, payload
    if access_token is not None:
        ath = (
            base64.urlsafe_b64encode(hashlib.sha256(access_token.encode()).digest())
            .decode()
            .rstrip("=")
        )
        assert payload["ath"] == ath, payload
    else:
        assert "ath" not in payload, payload


@app.route("/dpop")
def dpop():
    assert "Authorization" not in request.headers, request.headers.get("Authorization")
    verify_proof(request.headers["DPoP"], "GET", "http://localhost:8000/dpop")
    return "ok"


@app.route("/dpop-token")
def dpop_token():
    access_token = "my-access-token"
    assert request.headers["Authorization"] == f"DPoP {access_token}", request.headers[
        "Authorization"
    ]
    verify_proof(
        request.headers["DPoP"],
        "GET",
        "http://localhost:8000/dpop-token",
        access_token,
    )
    return "ok"
