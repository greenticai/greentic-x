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

verify_packaged_files() {
  crate_name="$1"
  package_list_file="$2"

  grep -Eq '^Cargo\.toml$' "$package_list_file" || {
    printf 'packaged crate %s is missing Cargo.toml\n' "$crate_name" >&2
    exit 1
  }
  grep -Eq '^README[^/]*$' "$package_list_file" || {
    printf 'packaged crate %s is missing a README file\n' "$crate_name" >&2
    exit 1
  }
  grep -Eq '^LICENSE[^/]*$' "$package_list_file" || {
    printf 'packaged crate %s is missing a LICENSE file\n' "$crate_name" >&2
    exit 1
  }
  grep -Eq '^src/' "$package_list_file" || {
    printf 'packaged crate %s is missing src/ contents\n' "$crate_name" >&2
    exit 1
  }
}

require_cmd cargo
require_cmd python3

crate_details_file="$(mktemp "${TMPDIR:-/tmp}/publishable-crates.XXXXXX")"
trap 'rm -f "$crate_details_file"' EXIT
python3 ci/publishable_crates.py --format details > "$crate_details_file"

step "cargo fmt"
cargo fmt --all -- --check

step "i18n validate"
bash tools/i18n.sh validate

step "cargo clippy"
cargo clippy --all-targets --all-features -- -D warnings

step "cargo test"
cargo test --all-features

step "cargo build"
cargo build --all-features

step "cargo doc"
cargo doc --no-deps --all-features

if [ -d "specs" ] && [ -d "catalog" ]; then
  step "specs and catalog"
  python3 ci/check_specs_catalog.py
fi

if [ -d "crates/gx" ]; then
  step "gx doctor"
  cargo run -p greentic-x -- doctor .
fi

if command -v greentic-pack >/dev/null 2>&1 && [ -d "packs" ]; then
  step "greentic-pack"
  bash ci/check_packs.sh
fi

if [ ! -s "$crate_details_file" ]; then
  step "Packaging checks"
  printf 'No publishable crates found in workspace.\n'
  exit 0
fi

step "Packaging checks"
while IFS='	' read -r crate_name manifest_path; do
  [ -n "$crate_name" ] || continue
  printf '\n-- %s\n' "$crate_name"
  if [ "$crate_name" != "greentic-x-types" ]; then
    printf 'Skipping local cargo package verification for %s; publish workflow validates ordered crates.io dry-runs after upstream dependencies are available.\n' "$crate_name"
    continue
  fi
  cargo package -p "$crate_name" --allow-dirty --no-verify
  cargo package -p "$crate_name" --allow-dirty

  package_list_file="$(mktemp "${TMPDIR:-/tmp}/package-list.XXXXXX")"
  cargo package -p "$crate_name" --allow-dirty --list > "$package_list_file"
  verify_packaged_files "$crate_name" "$package_list_file"
  rm -f "$package_list_file"
done < "$crate_details_file"
