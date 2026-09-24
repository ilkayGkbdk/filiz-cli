#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${FILIZ_REPO_URL:-https://github.com/ilkayGkbdk/filiz-cli.git}"
INSTALL_NAME="filiz"
GREEN='\033[38;5;150m'
OLIVE='\033[38;5;185m'
MUTED='\033[38;5;245m'
RED='\033[38;5;203m'
RESET='\033[0m'

clear_screen() {
  printf '\033[2J\033[H'
}

logo() {
  printf '%b\n' "${OLIVE}                 .-.-.${RESET}"
  printf '%b\n' "${OLIVE}              .-(     )-.${RESET}"
  printf '%b\n' "${OLIVE}             /    _    \\${RESET}"
  printf '%b\n' "${GREEN}            /   .' '.   \\${RESET}"
  printf '%b\n' "${GREEN}           /___/     \\___\\${RESET}"
  printf '%b\n' "${GREEN}              /  FILIZ  \\${RESET}"
  printf '%b\n' "${MUTED}             /___________\\${RESET}"
  printf '\n%b\n' "  ${MUTED}for betül, with love ♡${RESET}"
}

fail() {
  printf '\n%b\n' "${RED}✕ $1${RESET}" >&2
  printf '%b\n' "${MUTED}Kurulum tamamlanamadı. Yukarıdaki adımı kontrol edip tekrar deneyin.${RESET}" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 bulunamadı. Önce gerekli bağımlılığı kurun."
}

step() {
  local number="$1"
  local label="$2"
  printf '%b\n' "${OLIVE}[$number/4]${RESET} ${label}"
}

progress() {
  local message="$1"
  local i
  printf '    %s ' "$message"
  for i in 1 2 3 4 5 6 7 8 9 10; do
    printf '%b' "${GREEN}█${RESET}"
    sleep 0.06
  done
  printf '  %b\n' "${GREEN}OK${RESET}"
}

run_with_spinner() {
  local message="$1"
  shift
  local frames='|/-\\'
  local index=0
  "$@" >/tmp/filiz-install.log 2>&1 &
  local pid=$!
  while kill -0 "$pid" 2>/dev/null; do
    printf '\r    %s %b' "$message" "${GREEN}${frames:index++%4:1}${RESET}"
    sleep 0.12
  done
  if wait "$pid"; then
    printf '\r    %s %b\n' "$message" "${GREEN}OK${RESET}"
  else
    printf '\n'
    cat /tmp/filiz-install.log >&2
    fail "Cargo kurulumu başarısız oldu."
  fi
}

clear_screen
logo
printf '%b\n\n' "${MUTED}macOS terminal monitor kurulumu başlıyor...${RESET}"

step 1 "Ortam kontrol ediliyor"
require_command git
require_command cargo
progress "Git ve Cargo hazır"

step 2 "Filiz kaynak kodu alınıyor"
progress "Repository bağlantısı hazır"

step 3 "Filiz derleniyor ve kuruluyor"
run_with_spinner "Cargo ile Filiz kuruluyor" cargo install --git "$REPO_URL" --locked --force

step 4 "Kurulum doğrulanıyor"
if command -v "$INSTALL_NAME" >/dev/null 2>&1; then
  VERSION="$($INSTALL_NAME --version 2>/dev/null || true)"
  progress "${VERSION:-filiz komutu hazır}"
else
  printf '%b\n' "${MUTED}    Binary ~/.cargo/bin/filiz altına kuruldu.${RESET}"
  printf '%b\n' "${MUTED}    Terminal PATH ayarını yenileyin veya yeni terminal açın.${RESET}"
fi

printf '\n%b\n' "${GREEN}✓ Filiz hazır. Çalıştırmak için: filiz${RESET}"
printf '%b\n\n' "${MUTED}İyi izlemeler. ♡${RESET}"
