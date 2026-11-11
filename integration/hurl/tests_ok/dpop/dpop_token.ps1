Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

hurl --dpop --dpop-private-key tests_ok/dpop/key.pem tests_ok/dpop/dpop_token.hurl
