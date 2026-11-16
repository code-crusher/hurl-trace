#!/bin/bash
set -Eeuo pipefail

hurl --allow-commands --variable name=Bob --no-output tests_ok/command/command.hurl
