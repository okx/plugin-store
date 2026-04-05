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
        result = subprocess.run(["yq", ".build.lang // \"\"", yaml_path], capture_output=True, text=True)
        build_lang = result.stdout.strip()
        if build_lang in ("rust", "go"):
            needs_binary = True
        elif build_lang == "python":
            needs_pip = True
        elif build_lang in ("typescript", "node"):
            needs_npm = True

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

if not any([needs_onchainos, needs_binary, needs_pip, needs_npm]):
    print("No dependencies detected, skipping")
    sys.exit(0)

# Check what the DEVELOPER's pre-flight already installs
# Strip out any previous auto-injected block so we don't count our own injections
dev_skill_text = re.sub(
    r"## Pre-flight Dependencies \(auto-injected by Plugin Store CI\).*?---\n",
    "", skill_text, flags=re.DOTALL
)

# Only match actual install commands, not doc references like "Install via onchainos upgrade"
has_onchainos_install = bool(re.search(
    r"curl.*onchainos.*install\.sh|"
    r"skills add.*onchainos|"
    r"onchainos.*install\.sh\s*\|\s*sh|"
    r"brew install.*onchainos",
    dev_skill_text, re.I
))
has_binary_install = bool(re.search(r"curl.*releases/download|wget.*releases/download", dev_skill_text, re.I))
has_pip_install = bool(re.search(r"pip3? install", dev_skill_text, re.I))
has_npm_install = bool(re.search(r"npm install -g", dev_skill_text, re.I))

# Build injection block
parts = []

if needs_onchainos and not has_onchainos_install:
    parts.append("### Install onchainos CLI + Skills (auto-injected)\n\n```bash\n# 1. Install onchainos CLI\nonchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh\n\n# 2. Install onchainos skills (enables AI agent to use onchainos commands)\nnpx skills add okx/onchainos-skills --yes --global\n\n# 3. Install plugin-store skills (enables plugin discovery and management)\nnpx skills add okx/plugin-store --skill plugin-store --yes --global\n```\n")
    print("  Injecting: onchainos CLI + skills + plugin-store skill")

if needs_binary and not has_binary_install:
    block = f"""### Install {bin_name} binary (auto-injected)

```bash
if ! command -v {bin_name} >/dev/null 2>&1; then
  OS=$(uname -s | tr A-Z a-z)
  ARCH=$(uname -m)
  case "${{OS}}_${{ARCH}}" in
    darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
    darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
    linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    linux_aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  esac
  curl -fsSL "https://github.com/okx/plugin-store/releases/download/plugins/{name}@{version}/{bin_name}-${{TARGET}}" -o ~/.local/bin/{bin_name}
  chmod +x ~/.local/bin/{bin_name}
fi
```
"""
    parts.append(block)
    print(f"  Injecting: binary install ({bin_name})")

if needs_pip and not has_pip_install and src_repo:
    parts.append(f"### Install Python package (auto-injected)\n\n```bash\npip install git+https://github.com/{src_repo}@{src_commit} 2>/dev/null || pip3 install git+https://github.com/{src_repo}@{src_commit}\n```\n")
    print(f"  Injecting: pip install ({src_repo})")

if needs_npm and not has_npm_install and src_repo:
    parts.append(f"### Install npm package (auto-injected)\n\n```bash\nnpm install -g git+https://github.com/{src_repo}#{src_commit}\n```\n")
    print(f"  Injecting: npm install ({src_repo})")

if not parts:
    print("All dependencies already covered in existing pre-flight")
    sys.exit(0)

inject_block = "\n## Pre-flight Dependencies (auto-injected by Plugin Store CI)\n\n> Run once per session before first use. These checks ensure required tools are installed.\n\n" + "\n".join(parts) + "\n---\n\n"

# Inject into SKILL.md
fm_pattern = re.compile(r"^---\n.*?\n---\n", re.DOTALL)
fm_match = fm_pattern.match(skill_text)

if "auto-injected by Plugin Store CI" in skill_text:
    print("  Replacing existing auto-injected pre-flight...")
    skill_text = re.sub(
        r"## Pre-flight Dependencies \(auto-injected by Plugin Store CI\).*?---\n\n",
        inject_block,
        skill_text,
        flags=re.DOTALL,
    )
elif fm_match:
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
