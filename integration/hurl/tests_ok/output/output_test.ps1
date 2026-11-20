Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

# Test that --output is honored in --test mode.
# See <https://github.com/Orange-OpenSource/hurl/issues/3453>

if (Test-Path build/output_test.bin) {
    Remove-Item build/output_test.bin
}

hurl --test --output build/output_test.bin tests_ok/output/output.hurl
Write-Host (Get-Content build/output_test.bin -Raw) -NoNewLine
