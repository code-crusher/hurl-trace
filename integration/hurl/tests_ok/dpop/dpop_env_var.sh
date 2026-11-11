#!/bin/bash
set -Eeuo pipefail

export HURL_DPOP=true
export HURL_DPOP_PRIVATE_KEY=tests_ok/dpop/key.pem
hurl tests_ok/dpop/dpop.hurl
