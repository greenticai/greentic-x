#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${1:-all}"
AUTH_MODE="${AUTH_MODE:-auto}"
LOCALE="${LOCALE:-en}"
LANGS="${LANGS:-all}"
I18N_TRANSLATOR_BIN="${I18N_TRANSLATOR_BIN:-greentic-i18n-translator}"
BATCH_SIZE="${BATCH_SIZE:-500}"
LOCALES_PATH="${LOCALES_PATH:-crates/gx/i18n/locales.json}"

DEFAULT_EN_PATHS=(
  "crates/gx/i18n/en.json"
  "crates/gx/i18n/wizard/en.json"
)

usage() {
  cat <<'EOF'
Usage: tools/i18n.sh [translate|validate|status|all]

Environment overrides:
  EN_PATH=...                     English source file path (if set, only this catalog is processed)
  LANGS=...                       Target languages or `all` (default: all)
  AUTH_MODE=...                   Translator auth mode for translate (default: auto)
  LOCALE=...                      CLI locale used for translator output (default: en)
  I18N_TRANSLATOR_BIN=...         Translator executable name/path (default: greentic-i18n-translator)
  BATCH_SIZE=...                  Keys per translation request for translate (minimum: 500)
  LOCALES_PATH=...                JSON file with approved locales (default: crates/gx/i18n/locales.json)

Examples:
  tools/i18n.sh all
  AUTH_MODE=api-key tools/i18n.sh translate
  EN_PATH=crates/gx/i18n/wizard/en.json tools/i18n.sh validate
EOF
}

seed_missing_lang_files() {
  local en_path="$1"
  local i18n_dir
  i18n_dir="$(dirname "$en_path")"

  if [[ ! -f "$LOCALES_PATH" ]]; then
    echo "missing locales file: $LOCALES_PATH" >&2
    exit 2
  fi

  mkdir -p "$i18n_dir"
  python3 - "$LOCALES_PATH" "$i18n_dir" "$LANGS" <<'PY'
import json
import pathlib
import sys

locales_path = pathlib.Path(sys.argv[1])
i18n_dir = pathlib.Path(sys.argv[2])
langs_arg = sys.argv[3].strip()

locales = json.loads(locales_path.read_text())
if not isinstance(locales, list):
    raise SystemExit(f"{locales_path} must contain a JSON array")

if langs_arg == "all":
    target_langs = locales
else:
    target_langs = [lang.strip() for lang in langs_arg.split(",") if lang.strip()]

for lang in target_langs:
    if lang == "en":
        continue
    path = i18n_dir / f"{lang}.json"
    if path.exists():
        continue
    path.write_text("{\n}\n")
    print(f"seeded {path}")
PY
}

resolve_langs_arg() {
  python3 - "$LOCALES_PATH" "$LANGS" <<'PY'
import json
import pathlib
import sys

locales_path = pathlib.Path(sys.argv[1])
langs_arg = sys.argv[2].strip()

locales = json.loads(locales_path.read_text())
if not isinstance(locales, list):
    raise SystemExit(f"{locales_path} must contain a JSON array")

if langs_arg == "all":
    target_langs = [lang for lang in locales if lang != "en"]
else:
    target_langs = [lang.strip() for lang in langs_arg.split(",") if lang.strip() and lang.strip() != "en"]

print(",".join(target_langs))
PY
}

resolve_en_paths() {
  if [[ -n "${EN_PATH:-}" ]]; then
    printf '%s\n' "$EN_PATH"
    return 0
  fi

  printf '%s\n' "${DEFAULT_EN_PATHS[@]}"
}

require_i18n_translator() {
  if command -v "$I18N_TRANSLATOR_BIN" >/dev/null 2>&1; then
    return 0
  fi

  if [[ "$I18N_TRANSLATOR_BIN" != "greentic-i18n-translator" ]]; then
    cat >&2 <<EOF
missing required command: $I18N_TRANSLATOR_BIN
set I18N_TRANSLATOR_BIN to an installed translator path or leave it unset to auto-install greentic-i18n-translator
EOF
    exit 2
  fi

  echo "installing greentic-i18n-translator via cargo binstall"
  if ! command -v cargo-binstall >/dev/null 2>&1; then
    # Bootstrap from the prebuilt release binary: nothing is compiled, so a
    # cargo-binstall dependency raising its MSRV above the pinned toolchain
    # cannot break this step (cargo-binstall 1.22.0 did exactly that).
    binstall_ok=0
    for attempt in 1 2 3; do
      # `curl | bash` would hide a download failure: the pipeline reports
      # bash's status, and bash succeeds on empty input. Download, then run.
      binstall_installer="$(mktemp)"
      if curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh -o "$binstall_installer" && bash "$binstall_installer"; then
        hash -r
        if command -v cargo-binstall >/dev/null 2>&1; then binstall_ok=1; fi
      fi
      rm -f "$binstall_installer"
      if [ "$binstall_ok" -eq 1 ]; then break; fi
      sleep $((attempt * 5))
    done
    if [ "$binstall_ok" -ne 1 ]; then
      # Last release whose bundled lockfile still builds on 1.95.0.
      cargo install cargo-binstall --locked --version 1.21.1
    fi
  fi

  cargo binstall --no-confirm greentic-i18n-translator

  if command -v "$I18N_TRANSLATOR_BIN" >/dev/null 2>&1; then
    return 0
  fi

  echo "failed to install $I18N_TRANSLATOR_BIN" >&2
  exit 2
}

require_translate_batch_floor() {
  if ! [[ "$BATCH_SIZE" =~ ^[0-9]+$ ]]; then
    echo "BATCH_SIZE must be a positive integer, got: $BATCH_SIZE" >&2
    exit 2
  fi

  if (( BATCH_SIZE < 500 )); then
    echo "BATCH_SIZE must be at least 500, got: $BATCH_SIZE" >&2
    exit 2
  fi
}

run_translate_for() {
  local en_path="$1"
  local langs_arg="$2"
  echo "==> translate: $en_path"
  "$I18N_TRANSLATOR_BIN" \
    --locale "$LOCALE" \
    translate --langs "$langs_arg" --en "$en_path" --auth-mode "$AUTH_MODE" --batch-size "$BATCH_SIZE"
}

run_validate_for() {
  local en_path="$1"
  local langs_arg="$2"
  echo "==> validate: $en_path"
  "$I18N_TRANSLATOR_BIN" \
    --locale "$LOCALE" \
    validate --langs "$langs_arg" --en "$en_path"
}

run_status_for() {
  local en_path="$1"
  local langs_arg="$2"
  echo "==> status: $en_path"
  "$I18N_TRANSLATOR_BIN" \
    --locale "$LOCALE" \
    status --langs "$langs_arg" --en "$en_path"
}

if [[ "${MODE}" == "-h" || "${MODE}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ "$MODE" == "translate" || "$MODE" == "all" ]]; then
  require_translate_batch_floor
fi

require_i18n_translator

while IFS= read -r en_path; do
  local_langs="$(resolve_langs_arg)"

  if [[ ! -f "$en_path" ]]; then
    echo "missing English source map: $en_path" >&2
    exit 2
  fi

  seed_missing_lang_files "$en_path"

  case "$MODE" in
    translate)
      run_translate_for "$en_path" "$local_langs"
      ;;
    validate)
      run_validate_for "$en_path" "$local_langs"
      ;;
    status)
      run_status_for "$en_path" "$local_langs"
      ;;
    all)
      run_translate_for "$en_path" "$local_langs"
      run_validate_for "$en_path" "$local_langs"
      run_status_for "$en_path" "$local_langs"
      ;;
    *)
      echo "Unknown mode: $MODE" >&2
      usage
      exit 2
      ;;
  esac
done < <(resolve_en_paths)
