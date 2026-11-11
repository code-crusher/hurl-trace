Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

hurl --no-output --retry 10 --retry-interval 10s --retry-timeout 1s tests_ok/retry/retry_timeout_cli.hurl
