#!/usr/bin/env bash
# OKOne MR build job — Rust plugins.
# Mirrors plugin-store/.github/workflows/plugin-build.yml :: build-rust
# Compile Image suggestion: (offline)/okbase/rust:*-ossutil-okg*
set -euo pipefail

ROOT="${CI_PROJECT_DIR:-$(pwd)}"
TARGET_BRANCH="${CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-}"
if [ -n "${TARGET_BRANCH}" ]; then
  git fetch --depth=50 origin "${TARGET_BRANCH}" 2>/dev/null || true
  BASE_SHA="$(git merge-base "origin/${TARGET_BRANCH}" HEAD 2>/dev/null || true)"
else
  BASE_SHA="$(git rev-parse HEAD^1 2>/dev/null || true)"
fi
[ -z "${BASE_SHA}" ] && { echo "ERROR: cannot resolve diff base"; exit 1; }

CHANGED="$(git diff --name-only "${BASE_SHA}...HEAD" -- 'plugin-store/skills/' | head -100)"
PLUGIN_NAME="$(echo "${CHANGED}" | head -1 | cut -d'/' -f3)"
[ -n "${PLUGIN_NAME}" ] || { echo "no plugin changed under plugin-store/skills/, skipping"; exit 0; }
[[ "${PLUGIN_NAME}" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo "ERROR: invalid plugin name: ${PLUGIN_NAME}"; exit 1; }

PLUGIN_DIR="plugin-store/skills/${PLUGIN_NAME}"
YAML="${PLUGIN_DIR}/plugin.yaml"
[ -f "${YAML}" ] || { echo "no plugin.yaml in ${PLUGIN_DIR}, skipping"; exit 0; }

read_yaml() {
  YAML_FILE="${YAML}" KEY="$1" python3 - <<'PYEOF'
import os, yaml
y = os.environ["YAML_FILE"]
k = os.environ["KEY"]
with open(y) as f:
    d = yaml.safe_load(f) or {}
print(((d.get("build") or {}).get(k, "")) or "")
PYEOF
}

LANG="$(read_yaml lang)"
SOURCE_DIR="$(read_yaml source_dir)"
SOURCE_REPO="$(read_yaml source_repo)"
SOURCE_COMMIT="$(read_yaml source_commit)"
BINARY_NAME="$(read_yaml binary_name)"
[ -z "${SOURCE_DIR}" ] && SOURCE_DIR="."
[ "${LANG}" = "rust" ] || { echo "lang=${LANG} (not rust), skipping"; exit 0; }
[ -n "${BINARY_NAME}" ] || { echo "ERROR: build.binary_name missing in ${YAML}"; exit 1; }

WORK="${ROOT}/_build/${PLUGIN_NAME}"
rm -rf "${WORK}"; mkdir -p "${WORK}"
if [ -n "${SOURCE_REPO}" ] && [ -n "${SOURCE_COMMIT}" ]; then
  echo "external source: ${SOURCE_REPO}@${SOURCE_COMMIT}"
  git clone "https://github.com/${SOURCE_REPO}.git" "${WORK}/source"
  git -C "${WORK}/source" checkout "${SOURCE_COMMIT}"
else
  echo "local source: ${PLUGIN_DIR}"
  cp -r "${PLUGIN_DIR}" "${WORK}/source"
fi
SRC="${WORK}/source/${SOURCE_DIR}"
cd "${SRC}"

cargo fetch
( cargo install cargo-audit && cargo audit ) 2>&1 || true
cargo build --release

BIN="${SRC}/target/release/${BINARY_NAME}"
[ -f "${BIN}" ] || { echo "ERROR: binary not produced at ${BIN}"; exit 1; }
chmod +x "${BIN}"

OUT="${ROOT}/output"
mkdir -p "${OUT}"
cp "${BIN}" "${OUT}/${BINARY_NAME}"
sha256sum "${OUT}/${BINARY_NAME}"
echo "OK: rust build → ${OUT}/${BINARY_NAME}"
