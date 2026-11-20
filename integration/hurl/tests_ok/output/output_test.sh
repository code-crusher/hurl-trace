#!/bin/bash
set -Eeuo pipefail

# Test that --output is honored in --test mode.
# See <https://github.com/Orange-OpenSource/hurl/issues/3453>

rm -f build/output_test.bin

hurl --test --output build/output_test.bin tests_ok/output/output.hurl
cat build/output_test.bin
