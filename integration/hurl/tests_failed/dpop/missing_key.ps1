Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

hurl --dpop tests_failed/dpop/dpop.hurl
