#!/bin/bash
set -Eeuo pipefail

hurl tests_failed/assert_description/assert_description.hurl
