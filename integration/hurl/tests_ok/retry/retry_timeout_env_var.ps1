Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

$env:HURL_RETRY = 10
$env:HURL_RETRY_INTERVAL = "10s"
$env:HURL_RETRY_TIMEOUT = "1s"

hurl --no-output tests_ok/retry/retry_timeout_cli.hurl
