# OKOne MR Build Scripts

Per-language compile scripts for OKOne pipelines. Each script mirrors the
corresponding job in `plugin-store/.github/workflows/plugin-build.yml`.

## OKOne Job mapping

In OKOne, create one Compile Job per language (or one MR pipeline with
multiple Compile Jobs). For each Job:

| Field | Value |
|---|---|
| Compile Image | (see per-language note below) |
| Compile Shell Path | `.okone-cicd/compile/build-<lang>.sh` |
| Artifact Path | `output/` |

| Lang | Shell Path | Suggested Compile Image |
|---|---|---|
| Rust | `.okone-cicd/compile/build-rust.sh` | `(offline)/okbase/rust:*-ossutil-okg*` |
| Go | `.okone-cicd/compile/build-go.sh` | `(offline)/okbase/golang:1.22-*` (ask oncall for exact tag) |
| TypeScript | `.okone-cicd/compile/build-typescript.sh` | `(offline)/okbase/node:*` (script auto-installs Bun) |
| Node | `.okone-cicd/compile/build-node.sh` | `(offline)/okbase/node:*` (script auto-installs Bun) |
| Python | `.okone-cicd/compile/build-python.sh` | `(offline)/okbase/python:3.12-*` |

## How each script works

1. Resolve merge base against `CI_MERGE_REQUEST_TARGET_BRANCH_NAME`.
2. Find the changed plugin under `plugin-store/skills/<name>/`.
3. Parse `<plugin>/plugin.yaml` `build:` section for `lang`, `source_repo`,
   `source_commit`, `source_dir`, `binary_name`, `main`.
4. Skip silently (exit 0) if the changed plugin's `lang` doesn't match the script.
5. If `source_repo` + `source_commit` set: clone external repo at pinned SHA.
   Otherwise: build local plugin source.
6. Run language-specific build (cargo build / go build / bun compile / pip install).
7. Stage artifact at `output/<binary_name>` for OKOne to pick up.

## Required runtime tools

All scripts require: `git`, `python3` (with PyYAML), `bash`, `curl`, `sha256sum`.
The Compile Image must include those, plus the language toolchain.

If `python3 -c "import yaml"` fails in the chosen image, ask oncall to bake
PyYAML into the image, or prepend this line to the script:

```bash
pip install --quiet pyyaml || apt-get update && apt-get install -y python3-yaml
```

## Multi-plugin MRs

Scripts use the first changed plugin (`head -1`). If an MR touches multiple
plugins, only one is built per pipeline run. Match the GitHub Actions behavior
which does the same.
