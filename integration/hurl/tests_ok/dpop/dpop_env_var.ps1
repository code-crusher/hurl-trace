Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

$env:HURL_DPOP = "true"
$env:HURL_DPOP_PRIVATE_KEY = "tests_ok/dpop/key.pem"
hurl tests_ok/dpop/dpop.hurl
