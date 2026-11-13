Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

# A variable holding a duration with a unit is accepted for retry-interval.
hurl --variable interval=10ms --verbose tests_ok/retry/retry_interval_template.hurl

# A variable holding a duration without unit uses the default unit (milliseconds).
hurl --variable interval=10 --verbose tests_ok/retry/retry_interval_template.hurl
