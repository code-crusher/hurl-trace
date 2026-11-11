#!/bin/bash
set -Eeuo pipefail

export HURL_RETRY=10
export HURL_RETRY_INTERVAL=10s
export HURL_RETRY_TIMEOUT=1s

hurl --no-output tests_ok/retry/retry_timeout_cli.hurl
