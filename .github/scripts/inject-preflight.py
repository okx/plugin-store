#!/usr/bin/env python3
"""Scan for dependencies and inject pre-flight into SKILL.md."""
import sys, os, re, subprocess

name = sys.argv[1]
plugin_dir = sys.argv[2]

yaml_path = os.path.join(plugin_dir, "plugin.yaml")
skill_files = []
for root, dirs, files in os.walk(plugin_dir):
    for f in files:
        if f == "SKILL.md":
            skill_files.append(os.path.join(root, f))

if not skill_files:
    print("No SKILL.md found, skipping")
    sys.exit(0)

skill_file = skill_files[0]
skill_text = open(skill_file).read()

# Scan all text (SKILL + source code) for dependencies
all_text = skill_text
for ext in ["py", "rs", "go", "ts", "js"]:
    for root, dirs, files in os.walk(plugin_dir):
        for f in files:
            if f.endswith(f".{ext}"):
                all_text += open(os.path.join(root, f)).read()

# Detect dependencies
needs_onchainos = "onchainos" in all_text.lower()
needs_binary = False
needs_pip = False
needs_npm = False
build_lang = ""
bin_name = ""
version = "1.0.0"
src_repo = ""
src_commit = ""

if os.path.exists(yaml_path):
    try:
        import yaml
        with open(yaml_path) as f:
            plugin_data = yaml.safe_load(f) or {}
        build = plugin_data.get("build", {}) or {}
        build_lang = build.get("lang", "")
        if build_lang in ("rust", "go"):
            needs_binary = True
        elif build_lang == "python":
            needs_pip = True
        elif build_lang in ("typescript", "node"):
            needs_npm = True

        bin_name = build.get("binary_name", "") or name
        version = str(plugin_data.get("version", "1.0.0"))
        src_repo = build.get("source_repo", "")
        src_commit = build.get("source_commit", "")
    except Exception as e:
        print(f"  Warning: failed to parse {yaml_path}: {e}")
        # Fallback to yq if yaml module not available
        try:
            result = subprocess.run(["yq", ".build.lang // \"\"", yaml_path], capture_output=True, text=True)
            build_lang = result.stdout.strip()
            if build_lang in ("rust", "go"):
                needs_binary = True
            result = subprocess.run(["yq", ".build.binary_name // \"\"", yaml_path], capture_output=True, text=True)
            bin_name = result.stdout.strip() or name
            result = subprocess.run(["yq", ".version // \"1.0.0\"", yaml_path], capture_output=True, text=True)
            version = result.stdout.strip()
            result = subprocess.run(["yq", ".build.source_repo // \"\"", yaml_path], capture_output=True, text=True)
            src_repo = result.stdout.strip()
            result = subprocess.run(["yq", ".build.source_commit // \"\"", yaml_path], capture_output=True, text=True)
            src_commit = result.stdout.strip()
        except Exception:
            pass

# Strip ALL auto-injected content before detecting developer's own installs.
# This handles three cases:
# 1. Full section: "## Pre-flight Dependencies (auto-injected by Plugin Store CI)...---"
# 2. Standalone subsections: "### Install xxx (auto-injected)...```" copied without parent header
# 3. Any heading containing "(auto-injected)" followed by a code block

# First: remove the full CI-injected section if present
dev_skill_text = re.sub(
    r"## Pre-flight Dependencies \(auto-injected by Plugin Store CI\).*?---\n",
    "", skill_text, flags=re.DOTALL
)
# Second: remove any standalone (auto-injected) subsections that may exist outside the parent section
dev_skill_text = re.sub(
    r"###[^\n]*\(auto-injected[^\n]*\)\n(?:.*?\n)*?```\n(?:.*?\n)*?```\n*",
    "", dev_skill_text, flags=re.DOTALL
)

# Only skip onchainos injection if developer already has BOTH CLI install AND skills install
has_onchainos_cli = bool(re.search(
    r"curl.*onchainos.*install\.sh|"
    r"onchainos.*install\.sh\s*\|\s*sh|"
    r"brew install.*onchainos",
    dev_skill_text, re.I
))
has_onchainos_skills = bool(re.search(
    r"npx\s+skills\s+add\s+okx/onchainos-skills",
    dev_skill_text, re.I
))
has_onchainos_install = has_onchainos_cli and has_onchainos_skills
has_binary_install = bool(re.search(r"curl.*releases/download|wget.*releases/download", dev_skill_text, re.I))
has_pip_install = bool(re.search(r"pip3? install", dev_skill_text, re.I))
has_npm_install = bool(re.search(r"npm install -g", dev_skill_text, re.I))

# Build injection block
parts = []

# Always inject version check for ALL plugins (first block — runs before everything)
version_check_block = f"""### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. If an update is applied, re-read this SKILL.md before proceeding — the instructions may have changed.

```bash
# Check for skill updates (1-hour cache)
UPDATE_CACHE="$HOME/.plugin-store/update-cache/{name}"
CACHE_MAX=3600
LOCAL_VER="{version}"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/{name}/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{{print $2}}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: {name} v$LOCAL_VER -> v$REMOTE_VER. Updating..."
  npx skills add okx/plugin-store --skill {name} --yes --global 2>/dev/null || true
  echo "Updated {name} to v$REMOTE_VER. Please re-read this SKILL.md."
fi
```
"""
parts.append(version_check_block)
print(f"  Injecting: version check ({name} v{version})")

if needs_onchainos and not has_onchainos_install:
    parts.append("### Install onchainos CLI + Skills (auto-injected)\n\n```bash\n# 1. Install onchainos CLI\nonchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh\n\n# 2. Install onchainos skills (enables AI agent to use onchainos commands)\nnpx skills add okx/onchainos-skills --yes --global\n\n# 3. Install plugin-store skills (enables plugin discovery and management)\nnpx skills add okx/plugin-store --skill plugin-store --yes --global\n```\n")
    print("  Injecting: onchainos CLI + skills + plugin-store skill")

if needs_binary and not has_binary_install:
    block = f"""### Install {bin_name} binary + launcher (auto-injected)

```bash
# Install shared infrastructure (launcher + update checker, only once)
LAUNCHER="$HOME/.plugin-store/launcher.sh"
CHECKER="$HOME/.plugin-store/update-checker.py"
if [ ! -f "$LAUNCHER" ]; then
  mkdir -p "$HOME/.plugin-store"
  curl -fsSL "https://raw.githubusercontent.com/okx/plugin-store/main/scripts/launcher.sh" -o "$LAUNCHER" 2>/dev/null || true
  chmod +x "$LAUNCHER"
fi
if [ ! -f "$CHECKER" ]; then
  curl -fsSL "https://raw.githubusercontent.com/okx/plugin-store/main/scripts/update-checker.py" -o "$CHECKER" 2>/dev/null || true
fi

# Clean up old installation
rm -f "$HOME/.local/bin/{bin_name}" "$HOME/.local/bin/.{bin_name}-core" 2>/dev/null

# Download binary
OS=$(uname -s | tr A-Z a-z)
ARCH=$(uname -m)
EXT=""
case "${{OS}}_${{ARCH}}" in
  darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
  darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
  linux_x86_64)  TARGET="x86_64-unknown-linux-musl" ;;
  linux_i686)    TARGET="i686-unknown-linux-musl" ;;
  linux_aarch64) TARGET="aarch64-unknown-linux-musl" ;;
  linux_armv7l)  TARGET="armv7-unknown-linux-musleabihf" ;;
  mingw*_x86_64|msys*_x86_64|cygwin*_x86_64)   TARGET="x86_64-pc-windows-msvc"; EXT=".exe" ;;
  mingw*_i686|msys*_i686|cygwin*_i686)           TARGET="i686-pc-windows-msvc"; EXT=".exe" ;;
  mingw*_aarch64|msys*_aarch64|cygwin*_aarch64)  TARGET="aarch64-pc-windows-msvc"; EXT=".exe" ;;
esac
mkdir -p ~/.local/bin
curl -fsSL "https://github.com/okx/plugin-store/releases/download/plugins/{name}@{version}/{bin_name}-${{TARGET}}${{EXT}}" -o ~/.local/bin/.{bin_name}-core${{EXT}}
chmod +x ~/.local/bin/.{bin_name}-core${{EXT}}

# Symlink CLI name to universal launcher
ln -sf "$LAUNCHER" ~/.local/bin/{bin_name}

# Register version
mkdir -p "$HOME/.plugin-store/managed"
echo "{version}" > "$HOME/.plugin-store/managed/{bin_name}"
```
"""
    parts.append(block)
    print(f"  Injecting: binary install + launcher ({bin_name})")

if needs_pip and not has_pip_install and src_repo:
    parts.append(f"### Install Python package (auto-injected)\n\n```bash\npip install git+https://github.com/{src_repo}@{src_commit} 2>/dev/null || pip3 install git+https://github.com/{src_repo}@{src_commit}\n```\n")
    print(f"  Injecting: pip install ({src_repo})")

if needs_npm and not has_npm_install and src_repo:
    parts.append(f"### Install npm package (auto-injected)\n\n```bash\nnpm install -g git+https://github.com/{src_repo}#{src_commit}\n```\n")
    print(f"  Injecting: npm install ({src_repo})")

# Always inject install report (even if no other deps needed)
report_block = f"""### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/{name}"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  # Device fingerprint → SHA256 → 32-char device ID
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  # HMAC signature (obfuscated key, same as CLI binary)
  _K=$(echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | base64 -d 2>/dev/null || echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | openssl base64 -d)
  HMAC_SIG=$(echo -n "${{_K}}${{DEV_ID}}" | shasum -a 256 | head -c 8)
  DIV_ID="${{DEV_ID}}${{HMAC_SIG}}"
  unset _K
  # Report to Vercel stats
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \\
    -H "Content-Type: application/json" \\
    -d '{{"name":"{name}","version":"{version}"}}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \\
    -H "Content-Type: application/json" \\
    -d '{{"pluginName":"{name}","divId":"'"$DIV_ID"'"}}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```
"""
parts.append(report_block)
print(f"  Injecting: install report ({name})")

if len(parts) == 1 and not any([needs_onchainos, needs_binary, needs_pip, needs_npm]):
    # Only the report block, no other deps — still inject
    pass

inject_block = "\n## Pre-flight Dependencies (auto-injected by Plugin Store CI)\n\n> Run once per session before first use. These checks ensure required tools are installed.\n\n" + "\n".join(parts) + "\n---\n\n"

# Inject into SKILL.md
# First, strip any existing auto-injected content (full section or standalone blocks)
if "auto-injected" in skill_text:
    print("  Removing existing auto-injected pre-flight blocks...")
    # Remove full CI section
    skill_text = re.sub(
        r"\n?## Pre-flight Dependencies \(auto-injected by Plugin Store CI\).*?---\n\n?",
        "\n", skill_text, flags=re.DOTALL,
    )
    # Remove standalone (auto-injected) subsections outside the parent section
    skill_text = re.sub(
        r"\n?###[^\n]*\(auto-injected[^\n]*\)\n(?:.*?\n)*?```\n(?:.*?\n)*?```\n*",
        "\n", skill_text, flags=re.DOTALL,
    )
    # Clean up extra blank lines
    skill_text = re.sub(r"\n{3,}", "\n\n", skill_text)

# Now inject fresh pre-flight block after YAML frontmatter
fm_pattern = re.compile(r"^---\n.*?\n---\n", re.DOTALL)
fm_match = fm_pattern.match(skill_text)
if fm_match:
    insert_pos = fm_match.end()
    skill_text = skill_text[:insert_pos] + "\n" + inject_block + skill_text[insert_pos:]
else:
    skill_text = inject_block + skill_text

with open(skill_file, "w") as f:
    f.write(skill_text)

# Save injected content for PR comment
with open("/tmp/preflight_injected.txt", "w") as f:
    f.write(inject_block)

print(f"  SKILL.md patched: {skill_file}")
