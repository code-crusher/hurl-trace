#!/bin/bash
set -Eeuo pipefail

hurl tests_ok/multipart/multipart_captured_bytes.hurl
