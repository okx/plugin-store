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

echo "=== Rust toolchain (before) ==="
rustc --version 2>&1 || true
cargo --version 2>&1 || true
if command -v rustup >/dev/null 2>&1; then
  echo "=== rustup update stable ==="
  rustup update stable 2>&1 | tail -5 || true
  rustup default stable 2>&1 || true
  echo "=== Rust toolchain (after) ==="
  rustc --version 2>&1 || true
  cargo --version 2>&1 || true
else
  echo "rustup not found; using image's bundled Rust"
fi

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

# ════════════════════════════════════════════════════════════════════
#  GitHub release: sync changed files → tag → release → upload binary
# ════════════════════════════════════════════════════════════════════
if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "GITHUB_TOKEN not set, skipping GitHub release stage"
  exit 0
fi

GH_REPO="okx/plugin-store"
TARGET_TRIPLE="$(rustc -vV 2>/dev/null | awk '/host/{print $2; exit}')"
[ -z "${TARGET_TRIPLE}" ] && TARGET_TRIPLE="x86_64-unknown-linux-gnu"
ASSET="${BINARY_NAME}-${TARGET_TRIPLE}"
cp "${BIN}" "${OUT}/${ASSET}"

# Plugin version is at top level of plugin.yaml (not under build:).
PLUGIN_VERSION="$(awk -F'[: "]+' '/^version:/{print $2; exit}' "${ROOT}/${YAML}")"
[ -z "${PLUGIN_VERSION}" ] && { echo "ERROR: plugin.yaml has no version"; exit 1; }
TAG="plugins/${PLUGIN_NAME}@${PLUGIN_VERSION}"
echo "=== GitHub release: ${GH_REPO} ${TAG} ==="

# Clone target repo (token via URL env-expansion; mask in any echoed line)
GHWORK="${ROOT}/_github"
rm -rf "${GHWORK}"
{ git clone --depth 1 --quiet \
    "https://x-access-token:${GITHUB_TOKEN}@github.com/${GH_REPO}.git" \
    "${GHWORK}"; } 2>&1 | sed "s|${GITHUB_TOKEN}|<TOKEN>|g"

git -C "${GHWORK}" config user.email "okone-ci@okg.com"
git -C "${GHWORK}" config user.name "OKOne CI"

# Files to sync from `plugin-store/<x>` (GitLab) → `<x>` (GitHub):
# - registry.json
# - .claude-plugin/marketplace.json
# - skills/<this-plugin>/**
SYNC_LIST="$(git -C "${ROOT}" diff --name-only "${BASE_SHA}...HEAD" \
  | grep -E "^plugin-store/(registry\.json|\.claude-plugin/marketplace\.json|skills/${PLUGIN_NAME}/)" \
  || true)"

if [ -n "${SYNC_LIST}" ]; then
  echo "${SYNC_LIST}" > /tmp/sync_list.txt
  echo "syncing $(wc -l < /tmp/sync_list.txt) file(s):"
  while IFS= read -r src; do
    [ -z "${src}" ] && continue
    dst="${src#plugin-store/}"
    if [ -f "${ROOT}/${src}" ]; then
      mkdir -p "${GHWORK}/$(dirname "${dst}")"
      cp "${ROOT}/${src}" "${GHWORK}/${dst}"
      echo "  + ${dst}"
    else
      rm -f "${GHWORK}/${dst}"
      echo "  - ${dst}"
    fi
  done < /tmp/sync_list.txt

  if git -C "${GHWORK}" status --porcelain | grep -q .; then
    git -C "${GHWORK}" add -A
    git -C "${GHWORK}" commit -m "sync ${PLUGIN_NAME}@${PLUGIN_VERSION} from GitLab CI"
    { git -C "${GHWORK}" push origin HEAD:main; } 2>&1 | sed "s|${GITHUB_TOKEN}|<TOKEN>|g"
    echo "OK: pushed sync commit to ${GH_REPO}:main"
  else
    echo "no diff after copy (target already up-to-date)"
  fi
else
  echo "no syncable files in this MR"
fi

# Force-update tag (re-runnable)
git -C "${GHWORK}" tag -f "${TAG}"
{ git -C "${GHWORK}" push -f origin "refs/tags/${TAG}"; } 2>&1 | sed "s|${GITHUB_TOKEN}|<TOKEN>|g"
echo "OK: tag ${TAG}"

# Idempotent release: delete pre-existing one, then create fresh
GH_API="https://api.github.com/repos/${GH_REPO}"
EXIST="$(curl -fsS \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github+json" \
  "${GH_API}/releases/tags/${TAG}" 2>/dev/null \
  | sed -n 's/.*"id":[ ]*\([0-9][0-9]*\).*/\1/p' | head -1 || true)"
if [ -n "${EXIST}" ]; then
  echo "deleting existing release id=${EXIST}"
  curl -fsS -X DELETE \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    "${GH_API}/releases/${EXIST}" || true
fi

REL_BODY="$(printf '{"tag_name":"%s","name":"%s","draft":false,"prerelease":false}' \
  "${TAG}" "${PLUGIN_NAME} ${PLUGIN_VERSION}")"
REL_RESP="$(curl -fsS -X POST \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github+json" \
  -H "Content-Type: application/json" \
  -d "${REL_BODY}" \
  "${GH_API}/releases")"
UPLOAD_URL="$(echo "${REL_RESP}" | sed -n 's/.*"upload_url":[ ]*"\([^"{]*\).*/\1/p' | head -1)"
[ -z "${UPLOAD_URL}" ] && {
  echo "ERROR: failed to parse upload_url from release response"
  echo "${REL_RESP}" | head -20
  exit 1
}
echo "OK: release created"

# Upload asset
HTTP_CODE="$(curl -fsS -o /tmp/upload_resp.json -w "%{http_code}" -X POST \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@${OUT}/${ASSET}" \
  "${UPLOAD_URL}?name=${ASSET}")"
if [ "${HTTP_CODE}" = "201" ]; then
  ASSET_URL="$(sed -n 's/.*"browser_download_url":[ ]*"\([^"]*\).*/\1/p' /tmp/upload_resp.json | head -1)"
  echo "OK: uploaded ${ASSET}"
  echo "    ${ASSET_URL}"
else
  echo "ERROR: asset upload returned HTTP ${HTTP_CODE}"
  head -20 /tmp/upload_resp.json
  exit 1
fi
