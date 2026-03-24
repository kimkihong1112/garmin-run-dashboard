#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PYTHON_BIN="${PYTHON_BIN:-/opt/homebrew/bin/python3.12}"
VENV_DIR="${ROOT_DIR}/.venv-garmin"

if [ ! -x "${PYTHON_BIN}" ]; then
  echo "Python 3.12 was not found at ${PYTHON_BIN}."
  echo "Install it first or run with PYTHON_BIN pointing to a compatible interpreter."
  exit 1
fi

"${PYTHON_BIN}" -m venv "${VENV_DIR}"
"${VENV_DIR}/bin/pip" install --upgrade pip
"${VENV_DIR}/bin/pip" install garminconnect

echo "Garmin adapter environment is ready at ${VENV_DIR}."
