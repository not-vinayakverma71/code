#!/usr/bin/env bash
set -euo pipefail

# Automated UI streaming test for Floem example: gemini_chatbot
# Requirements:
#   - env GEMINI_API_KEY must be set (no hardcoded keys)
# Optional:
#   - first argument: prompt to force multi-chunk streaming

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="${SCRIPT_DIR}/.."
ROOT_DIR="$(cd "${APP_DIR}/.." && pwd)"
LOG_FILE="${APP_DIR}/target/gemini_ui_test.log"

# Load GEMINI_API_KEY from repo .env files if not already set
if [[ -z "${GEMINI_API_KEY:-}" ]]; then
  # Prefer lapce-ai/.env if present
  if [[ -f "${ROOT_DIR}/lapce-ai/.env" ]]; then
    set -a
    # shellcheck disable=SC1090
    . "${ROOT_DIR}/lapce-ai/.env"
    set +a
  fi
fi

# Fallback: check lapce-app/.env
if [[ -z "${GEMINI_API_KEY:-}" && -f "${APP_DIR}/.env" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${APP_DIR}/.env"
  set +a
fi

if [[ -z "${GEMINI_API_KEY:-}" ]]; then
  echo "ERROR: GEMINI_API_KEY is not set (tried lapce-ai/.env and lapce-app/.env)." >&2
  exit 2
fi

PROMPT_DEFAULT="Write a detailed 1500+ word essay to force streaming across multiple SSE events. Include sections, bullet points, and inline code samples."
PROMPT="${1:-$PROMPT_DEFAULT}"
TO_SECONDS="${GEMINI_UI_TIMEOUT_SEC:-180}"

pushd "$APP_DIR" >/dev/null

# Clean previous log
mkdir -p target
: > "$LOG_FILE"

# Run the Floem example in auto-test mode
# The app will quit itself when streaming completes and print a single TEST_RESULT line
GEMINI_UI_TEST=1 GEMINI_UI_PROMPT="$PROMPT" \
  timeout -k 10 "${TO_SECONDS}s" \
  cargo run --example gemini_chatbot | tee -a "$LOG_FILE"

# Parse test result from log
RESULT_LINE="$(grep -E "^TEST_RESULT " "$LOG_FILE" | tail -n 1 || true)"
if [[ -z "$RESULT_LINE" ]]; then
  echo "ERROR: No TEST_RESULT line found in log. Check $LOG_FILE" >&2
  exit 1
fi

echo "Last result: $RESULT_LINE"
if echo "$RESULT_LINE" | grep -q "pass=true"; then
  echo "UI streaming test: PASS"
  exit 0
else
  echo "UI streaming test: FAIL"
  exit 1
fi
