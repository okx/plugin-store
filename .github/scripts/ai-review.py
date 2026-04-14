#!/usr/bin/env python3
"""Run AI code review on a plugin submission via Claude API."""
import sys, os, json, glob, subprocess

name = sys.argv[1]
plugin_dir = sys.argv[2]
api_key = os.environ.get("ANTHROPIC_API_KEY", "")

if not api_key:
    print("No ANTHROPIC_API_KEY, skipping AI review")
    sys.exit(0)

# Build plugin content
yaml_path = os.path.join(plugin_dir, "plugin.yaml")
yaml_content = open(yaml_path).read() if os.path.exists(yaml_path) else ""

skill_content = ""
if os.path.exists("/tmp/skill_content.txt"):
    skill_content = open("/tmp/skill_content.txt").read()

# Collect source code — supports both Mode A (local) and Mode B (external repo)
source_files = []
source_dir = plugin_dir

# Mode B: if plugin.yaml specifies source_repo, clone and use external source
if os.path.exists(yaml_path):
    try:
        src_repo = subprocess.run(["yq", ".build.source_repo // \"\"", yaml_path],
                                  capture_output=True, text=True).stdout.strip()
        src_commit = subprocess.run(["yq", ".build.source_commit // \"\"", yaml_path],
                                    capture_output=True, text=True).stdout.strip()
        src_subdir = subprocess.run(["yq", ".build.source_dir // \".\"", yaml_path],
                                    capture_output=True, text=True).stdout.strip()
        if src_repo and src_commit:
            ext_dir = f"/tmp/ai-review-source-{name}"
            clone_result = subprocess.run(
                ["git", "clone", "--depth", "1", f"https://github.com/{src_repo}.git", ext_dir],
                capture_output=True, text=True, timeout=60
            )
            if clone_result.returncode == 0:
                subprocess.run(["git", "-C", ext_dir, "fetch", "--depth", "1", "origin", src_commit],
                              capture_output=True, text=True, timeout=60)
                subprocess.run(["git", "-C", ext_dir, "checkout", src_commit],
                              capture_output=True, text=True, timeout=10)
                if src_subdir and src_subdir != ".":
                    source_dir = os.path.join(ext_dir, src_subdir)
                else:
                    source_dir = ext_dir
                print(f"  Mode B: cloned {src_repo}@{src_commit[:12]} → {source_dir}")
            else:
                print(f"  Mode B: clone failed, falling back to local source")
    except Exception as e:
        print(f"  Mode B detection error: {e}")

for ext in ["py", "rs", "go", "ts", "js", "json", "yaml", "yml", "md", "html"]:
    for path in glob.glob(f"{source_dir}/**/*.{ext}", recursive=True):
        if ".git" not in path:
            try:
                content = open(path).read()
                rel = os.path.relpath(path, source_dir)
                source_files.append(f"## {rel}\n```{ext}\n{content}\n```\n")
            except:
                pass

source_block = "\n".join(source_files) if source_files else "(no source code in submission)"

# Build review prompt
prompt = f"""Review this plugin submission for the OKX Plugin Store.

Plugin name: {name}

Evaluate on these dimensions:
1. Security — any malicious code, prompt injection, credential theft, rug-pull patterns?
2. Functionality — does the SKILL.md correctly describe what the code does?
3. Safety defaults — are dangerous operations (trading, signing) paused/paper-mode by default?
4. Code quality — any obvious bugs, unsafe patterns, or missing error handling?
5. onchainos compliance — if on-chain operations exist, do they use onchainos CLI?

Output format:
Quality Score: <0-100>
Recommendation: <Ready to merge | Needs changes | Merge with caveats>

Then provide a brief report (under 500 words) covering the 5 dimensions above.

=== plugin.yaml ===
{yaml_content}

=== SKILL.md ===
{skill_content[:5000]}

=== Source Code ===
{source_block[:10000]}
"""

with open("/tmp/review_prompt.txt", "w") as f:
    f.write(prompt)

# Call Claude API
import urllib.request, urllib.error

req_body = json.dumps({
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 2048,
    "messages": [{"role": "user", "content": prompt}]
}).encode()

req = urllib.request.Request(
    "https://api.anthropic.com/v1/messages",
    data=req_body,
    headers={
        "Content-Type": "application/json",
        "x-api-key": api_key,
        "anthropic-version": "2023-06-01",
    },
)

try:
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read())
        review = data["content"][0]["text"]
        model = data.get("model", "unknown")
        tokens_in = data.get("usage", {}).get("input_tokens", 0)
        tokens_out = data.get("usage", {}).get("output_tokens", 0)

    with open("/tmp/ai_review.md", "w") as f:
        f.write(review)

    # Extract score and recommendation
    score = "N/A"
    rec = "manual"
    for line in review.split("\n"):
        if "Quality Score:" in line:
            import re
            m = re.search(r"(\d+)", line)
            if m:
                score = m.group(1)
        if "Ready to merge" in line:
            rec = "ready"
        elif "Needs changes" in line:
            rec = "changes"
        elif "caveats" in line.lower():
            rec = "caveats"

    with open("/tmp/ai_review_meta.json", "w") as f:
        json.dump({"score": score, "rec": rec, "model": model, "tokens": f"~{tokens_in}+{tokens_out}"}, f)

    print(f"AI Review complete: Score={score}, Rec={rec}, Model={model}")

except urllib.error.HTTPError as e:
    error_body = e.read().decode()
    error_msg = json.loads(error_body).get("error", {}).get("message", str(e))
    print(f"API error: {error_msg}")

    with open("/tmp/ai_review.md", "w") as f:
        f.write(f"AI review failed: {error_msg}")
    with open("/tmp/ai_review_meta.json", "w") as f:
        json.dump({"score": "N/A", "rec": "manual", "model": "unavailable", "tokens": "N/A"}, f)

except Exception as e:
    print(f"Error: {e}")
    with open("/tmp/ai_review.md", "w") as f:
        f.write(f"AI review error: {e}")
    with open("/tmp/ai_review_meta.json", "w") as f:
        json.dump({"score": "N/A", "rec": "manual", "model": "unavailable", "tokens": "N/A"}, f)
