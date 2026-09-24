#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${FILIZ_REPO_URL:-https://github.com/ilkayGkbdk/filiz-cli.git}"
RAW_URL="${FILIZ_RAW_URL:-https://raw.githubusercontent.com/ilkayGkbdk/filiz-cli/main}"
INSTALL_NAME="filiz"
ESC=$'\033'
RESET="${ESC}[0m"
GREEN="${ESC}[38;5;150m"
OLIVE="${ESC}[38;5;185m"
MUTED="${ESC}[38;5;245m"
WHITE="${ESC}[38;5;255m"
BG="${ESC}[48;2;10;14;12m"
BOX_WIDTH=76
LOGO_FILE=""

cleanup() {
  printf '%s' "${RESET}${ESC}[?25h${ESC}[?1049l"
  [[ -n "${LOGO_FILE}" ]] && rm -f -- "${LOGO_FILE}"
}

trap cleanup EXIT
trap 'exit 130' INT TERM

terminal_size() {
  COLUMNS="$(tput cols 2>/dev/null || printf '100')"
  LINES="$(tput lines 2>/dev/null || printf '30')"
  LEFT=$(( (COLUMNS - BOX_WIDTH) / 2 ))
  (( LEFT < 0 )) && LEFT=0
  TOP=$(( (LINES - 24) / 2 ))
  (( TOP < 1 )) && TOP=1
}

indent() { printf '%*s' "$LEFT" ''; }

box_text() {
  local text="$1"
  indent
  printf '%s│%s  %s\n' "$OLIVE" "$RESET" "$text"
}

fallback_logo() {
  printf '%b' "${OLIVE}                  .  .${RESET}\n"
  printf '%b' "${OLIVE}               .  /\\  .${RESET}\n"
  printf '%b' "${GREEN}              /\\ /  \\ /\\${RESET}\n"
  printf '%b' "${GREEN}             /  \\____/  \\${RESET}\n"
  printf '%b' "${GREEN}                \\FILIZ/${RESET}\n"
}

load_logo() {
  LOGO_FILE="$(mktemp -t filiz-logo)"
  if command -v curl >/dev/null 2>&1 && curl -fsSL --max-time 8 "${RAW_URL}/assets/logo.ansi" -o "$LOGO_FILE"; then
    return
  fi
  : >"$LOGO_FILE"
}

logo_block() {
  local line_content
  if [[ -s "$LOGO_FILE" ]]; then
    while IFS= read -r line_content; do
      indent
      printf '%s│%s  %s\n' "$OLIVE" "$RESET" "$line_content"
    done <"$LOGO_FILE"
  else
    while IFS= read -r line_content; do
      indent
      printf '%s│%s  %b\n' "$OLIVE" "$RESET" "$line_content"
    done < <(fallback_logo)
  fi
}

render() {
  local active="$1"
  local message="$2"
  local state="$3"
  local marker="${4:-}"
  terminal_size
  printf '%s%s%s' "${ESC}[?1049h${ESC}[?25l${ESC}[2J${ESC}[H" "$BG" "$WHITE"
  printf '\n%.0s' $(seq 1 "$TOP")
  indent; printf '%s╭%*s╮%s\n' "$OLIVE" $((BOX_WIDTH - 2)) '' "$RESET"
  logo_block
  box_text ""
  box_text "for betül, with love ♡"
  box_text ""
  box_text "${MUTED}macOS terminal monitor installer${RESET}"
  box_text ""
  box_text "${OLIVE}[1/4]${RESET}  Environment       $([[ $active -ge 1 ]] && printf '%s✓%s' "$GREEN" "$RESET" || printf '·')"
  box_text "${OLIVE}[2/4]${RESET}  Repository         $([[ $active -ge 2 ]] && printf '%s✓%s' "$GREEN" "$RESET" || printf '·')"
  box_text "${OLIVE}[3/4]${RESET}  Build & install    $([[ $active -ge 3 ]] && printf '%s✓%s' "$GREEN" "$RESET" || printf '·')"
  box_text "${OLIVE}[4/4]${RESET}  Verify              $([[ $active -ge 4 ]] && printf '%s✓%s' "$GREEN" "$RESET" || printf '·')"
  box_text ""
  box_text "${GREEN}${marker}${RESET}  ${message}"
  box_text ""
  indent; printf '%s╰%*s╯%s\n' "$OLIVE" $((BOX_WIDTH - 2)) '' "$RESET"
  printf '%s' "$RESET"
}

fail() {
  local message="$1"
  render 3 "$message" "ERROR" "✕"
  sleep 1
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 bulunamadı; önce bağımlılığı kurun."
}

run_with_spinner() {
  local message="$1"
  shift
  local frames='|/-\\'
  local index=0
  local log_file
  log_file="$(mktemp -t filiz-install)"
  "$@" >"$log_file" 2>&1 &
  local pid=$!
  while kill -0 "$pid" 2>/dev/null; do
    render 3 "$message" "RUNNING" "${frames:index%4:1}"
    index=$((index + 1))
    sleep 0.12
  done
  if wait "$pid"; then
    rm -f -- "$log_file"
  else
    cat "$log_file" >&2
    rm -f -- "$log_file"
    fail "Cargo kurulumu başarısız oldu."
  fi
}

load_logo
render 0 "Kurulum hazırlanıyor..." "START" "·"
sleep 0.35

require_command git
require_command cargo
render 1 "Git ve Cargo hazır" "READY" "✓"
sleep 0.35

render 2 "Filiz repository bağlantısı hazır" "READY" "✓"
sleep 0.35

run_with_spinner "Filiz derleniyor ve kuruluyor" cargo install --git "$REPO_URL" --locked --force

if command -v "$INSTALL_NAME" >/dev/null 2>&1; then
  VERSION="$($INSTALL_NAME --version 2>/dev/null || true)"
  render 4 "${VERSION:-filiz komutu hazır}" "READY" "✓"
else
  render 4 "Binary ~/.cargo/bin/filiz altına kuruldu; PATH'i yenileyin" "READY" "✓"
fi
sleep 1

render 4 "Filiz hazır — çalıştırmak için: filiz" "COMPLETE" "✓"
sleep 1.2
