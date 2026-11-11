#!/bin/bash
set -Eeuo pipefail

hurl --dpop tests_failed/dpop/dpop.hurl
