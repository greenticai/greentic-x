#!/usr/bin/env bash
set -euo pipefail

repo_root="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$repo_root"

step() {
  printf '\n==> %s\n' "$1"
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$1" >&2
    exit 1
  fi
}

require_cmd greentic-pack

set -- packs/*/pack.yaml
if [ "$1" = 'packs/*/pack.yaml' ]; then
  step "Pack checks"
  printf 'No source packs found.\n'
  exit 0
fi

step "Pack checks"
for pack_manifest in "$@"; do
  pack_dir="$(dirname "$pack_manifest")"
  pack_name="$(basename "$pack_dir")"
  printf '\n-- %s\n' "$pack_name"
  greentic-pack build --in "$pack_dir"
  greentic-pack doctor "$pack_dir/dist/$pack_name.gtpack"
done
