#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
[[ "$(git rev-parse --show-toplevel)" == "$PWD" ]] || { echo 'A real checkout is required.' >&2; exit 2; }
existing=$(git config --get core.hooksPath || true)
[[ -z "$existing" || "$existing" == .githooks ]] || { echo "Existing hooks path preserved: $existing" >&2; exit 2; }
git config --local core.hooksPath .githooks
echo 'Enabled local staged-path guard; provider hooks and physical tests are not enabled.'
