#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${1:-all}"
AUTH_MODE="${AUTH_MODE:-auto}"
LOCALE="${LOCALE:-en}"
LANGS="${LANGS:-all}"
I18N_TRANSLATOR_MANIFEST="${I18N_TRANSLATOR_MANIFEST:-../greentic-i18n/Cargo.toml}"
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
  I18N_TRANSLATOR_MANIFEST=...    Path to greentic-i18n Cargo.toml
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
  cargo run --manifest-path "$I18N_TRANSLATOR_MANIFEST" -p greentic-i18n-translator -- \
    --locale "$LOCALE" \
    translate --langs "$langs_arg" --en "$en_path" --auth-mode "$AUTH_MODE" --batch-size "$BATCH_SIZE"
}

run_validate_for() {
  local en_path="$1"
  local langs_arg="$2"
  echo "==> validate: $en_path"
  cargo run --manifest-path "$I18N_TRANSLATOR_MANIFEST" -p greentic-i18n-translator -- \
    --locale "$LOCALE" \
    validate --langs "$langs_arg" --en "$en_path"
}

run_status_for() {
  local en_path="$1"
  local langs_arg="$2"
  echo "==> status: $en_path"
  cargo run --manifest-path "$I18N_TRANSLATOR_MANIFEST" -p greentic-i18n-translator -- \
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
