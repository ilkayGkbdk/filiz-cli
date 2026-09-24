#!/usr/bin/env bash

set -euo pipefail

removed=0
managed_binary="${HOME}/.local/bin/filiz"

if command -v cargo >/dev/null 2>&1; then
  if cargo uninstall filiz >/dev/null 2>&1; then
    echo "Removed Filiz from Cargo's bin directory."
    removed=1
  fi
fi

if [[ -f "${managed_binary}" || -L "${managed_binary}" ]]; then
  rm -f -- "${managed_binary}"
  echo "Removed ${managed_binary}."
  removed=1
fi

if [[ "${removed}" -eq 0 ]]; then
  echo "Filiz is not installed in the supported locations."
else
  echo "Filiz has been uninstalled."
fi
