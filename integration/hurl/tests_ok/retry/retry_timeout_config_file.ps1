Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

$env:XDG_CONFIG_HOME = "$PSScriptRoot/config_timeout"

hurl --no-output tests_ok/retry/retry_timeout_cli.hurl
