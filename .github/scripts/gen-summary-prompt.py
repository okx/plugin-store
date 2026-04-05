#!/usr/bin/env python3
"""Generate the Claude API prompt for SUMMARY.md + SKILL_SUMMARY.md."""
import sys, os

name = sys.argv[1]
plugin_dir = sys.argv[2]

yaml_path = os.path.join(plugin_dir, "plugin.yaml")
readme_path = os.path.join(plugin_dir, "README.md")

yaml_content = open(yaml_path).read() if os.path.exists(yaml_path) else ""
readme_content = open(readme_path).read() if os.path.exists(readme_path) else ""
skill_content = ""
if os.path.exists("/tmp/skill_content.txt"):
    skill_content = "".join(open("/tmp/skill_content.txt").readlines()[:500])

prompt = f"""You are generating documentation for plugin "{name}".

Given the SKILL.md, README.md, and plugin.yaml below, generate TWO markdown files.

Output exactly two sections separated by the line: ---SEPARATOR---

FIRST section is SUMMARY.md:
# {name}
<one sentence description>
## Highlights
- <feature 1>
- <feature 2>
...up to 8 highlights

SECOND section is SKILL_SUMMARY.md:
# {name} -- Skill Summary
## Overview
<one paragraph functional overview>
## Usage
<how to use/start, 1-3 sentences>
## Commands
<table or list of CLI commands. Write "This is a reference skill with no CLI commands." if none>
## Triggers
<when an AI agent should activate this skill, 1-2 sentences>

=== INPUT ===

plugin.yaml:
{yaml_content}

README.md:
{readme_content}

SKILL.md:
{skill_content}
"""

with open("/tmp/prompt.txt", "w") as f:
    f.write(prompt)
print(f"Prompt written: {len(prompt)} chars")
