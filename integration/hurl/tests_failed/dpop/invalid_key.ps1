Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

hurl --dpop --dpop-private-key tests_failed/dpop/invalid_key.pem tests_failed/dpop/dpop.hurl
