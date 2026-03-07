#!/usr/bin/env bash
set -euo pipefail

required_version="${GREENTIC_PACK_VERSION:-0.4.109}"

if command -v greentic-pack >/dev/null 2>&1; then
  installed_version="$(greentic-pack --version | awk '{print $2}')"
  if [ "$installed_version" = "$required_version" ]; then
    printf 'greentic-pack %s already installed\n' "$installed_version"
    exit 0
  fi
fi

if ! command -v cargo-binstall >/dev/null 2>&1; then
  cargo install cargo-binstall --locked
fi

cargo binstall --no-confirm "greentic-pack@$required_version"
