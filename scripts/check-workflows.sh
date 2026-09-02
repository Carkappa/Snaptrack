#!/usr/bin/env bash
#
# Validates the workflow YAML before it is pushed.
#
# A broken workflow file cannot report its own breakage: GitHub cannot parse
# it, so it never runs the checks inside it and the run shows up as the file
# path rather than the workflow name. That is the one failure CI structurally
# cannot catch, which is why this runs locally.
#
# Needs pyyaml: pip install pyyaml
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'CHECK'
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    print("pyyaml not installed - skipping (pip install pyyaml)", file=sys.stderr)
    sys.exit(0)

failed = False
files = sorted(Path(".github/workflows").glob("*.yml"))

for path in files:
    try:
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as e:
        print(f"{path}: invalid YAML\n  {e}", file=sys.stderr)
        failed = True
        continue

    if not isinstance(doc, dict) or "jobs" not in doc:
        print(f"{path}: no jobs - is it really a workflow?", file=sys.stderr)
        failed = True
        continue

    for job_name, job in doc["jobs"].items():
        for step in job.get("steps", []):
            # A step that neither runs nor uses anything is usually a
            # comment that stopped being a comment.
            if "run" not in step and "uses" not in step:
                print(
                    f"{path}: job '{job_name}' has a step with neither run nor uses: {step}",
                    file=sys.stderr,
                )
                failed = True

if failed:
    sys.exit(1)

print(f"All {len(files)} workflow files parse.")
CHECK
