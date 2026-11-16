Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

hurl --allow-commands --variable name=Bob --no-output tests_ok/command/command.hurl
