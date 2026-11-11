#!/bin/bash
set -Eeuo pipefail

hurl tests_ok/retry/retry_timeout.hurl
