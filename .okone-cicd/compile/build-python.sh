#!/usr/bin/env bash
# OKOne MR build job — Python plugins (pip-install validation, no binary).
# Mirrors plugin-store/.github/workflows/plugin-build.yml :: build-python
# Compile Image suggestion: (offline)/okbase/python:3.12-* or similar.
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
  awk -v key="$1" '
    /^build:[ \t]*$/ { in_build=1; next }
    /^[^ \t#]/        { in_build=0 }
    in_build && $0 ~ "^[ \t]+"key"[ \t]*:" {
      sub(/^[^:]*:[ \t]*/, "")
      sub(/^"/, ""); sub(/"$/, "")
      print; exit
    }
  ' "${YAML}"
}

LANG="$(read_yaml lang)"
SOURCE_DIR="$(read_yaml source_dir)"
SOURCE_REPO="$(read_yaml source_repo)"
SOURCE_COMMIT="$(read_yaml source_commit)"
BINARY_NAME="$(read_yaml binary_name)"
BUILD_MAIN="$(read_yaml main)"
[ -z "${SOURCE_DIR}" ] && SOURCE_DIR="."
[ "${LANG}" = "python" ] || { echo "lang=${LANG} (not python), skipping"; exit 0; }

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

if [ -f pyproject.toml ]; then
  python3 - <<'PYEOF' || true
import tomllib
try:
    with open("pyproject.toml", "rb") as f:
        d = tomllib.load(f)
    print("requires-python:", d.get("project", {}).get("requires-python", "not specified"))
except Exception as e:
    print("could not parse pyproject.toml:", e)
PYEOF
fi

pip install pip-audit
if ! pip install -e . 2>/dev/null && ! pip install -r requirements.txt 2>/dev/null && ! pip install . ; then
  echo "ERROR: pip install failed"
  exit 1
fi

pip-audit 2>&1 || true

if [ -n "${BUILD_MAIN}" ] && python3 "${BUILD_MAIN}" --help >/dev/null 2>&1; then
  echo "entry point ${BUILD_MAIN} responds to --help"
elif [ -n "${BINARY_NAME}" ] && command -v "${BINARY_NAME}" >/dev/null 2>&1; then
  echo "installed command ${BINARY_NAME} found in PATH"
else
  echo "WARNING: could not verify entry point, but pip install succeeded"
fi

OUT="${ROOT}/output"
mkdir -p "${OUT}"
PKG_NAME="$(python3 - <<'PYEOF' 2>/dev/null || true
import tomllib
try:
    with open("pyproject.toml", "rb") as f:
        print(tomllib.load(f).get("project", {}).get("name", ""))
except Exception:
    print("")
PYEOF
)"
[ -n "${PKG_NAME}" ] && pip show "${PKG_NAME}" > "${OUT}/${PKG_NAME}.pip-info.txt" 2>/dev/null || true
echo "OK: python validate"
