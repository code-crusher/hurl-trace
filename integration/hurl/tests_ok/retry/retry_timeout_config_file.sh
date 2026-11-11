#!/bin/bash
set -Eeuo pipefail

XDG_CONFIG_HOME=$(dirname "$0")/config_timeout
export XDG_CONFIG_HOME

hurl --no-output tests_ok/retry/retry_timeout_cli.hurl
