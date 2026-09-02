#!/usr/bin/env bash
#
# The test harness mocks `invoke` by name and returns hand-written objects
# shaped like the Rust types. Nothing keeps them in step: rename a field in
# Rust and the harness keeps returning the old one, so the UI keeps working
# in tests and breaks in the built app. That is the same blind spot that let
# two unregistered commands ship for several releases.
#
# This compares the fields of the Rust types the frontend consumes against
# the keys the harness returns for them. A lint, not a parser: it reports
# fields present in Rust that the harness never produces.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'CHECK'
import re
import sys
from pathlib import Path

rust = (
    Path("src-tauri/src/commands.rs").read_text(encoding="utf-8")
    + "\n"
    + Path("src-tauri/src/models.rs").read_text(encoding="utf-8")
)
harness = Path("tests/ui-harness.html").read_text(encoding="utf-8")

# Rust struct -> the harness handler that has to produce it.
PAIRS = {
    "OllamaStatus": "ollama_status",
    "ImportSummary": "import_applications",
    "ExtractionProvider": "get_extraction_providers",
    "StatusDef": "get_status_defs",
}

NEXT_ENTRY = "\n      " + r"\w+:"


def rust_fields(name):
    m = re.search(r"pub struct " + name + r"\s*\{(.*?)\n\}", rust, re.S)
    if not m:
        return None
    fields = []
    for line in m.group(1).split("\n"):
        line = line.strip()
        # Skip attributes; a #[serde(flatten)] field contributes its own keys.
        if line.startswith("#") or not line.startswith("pub "):
            continue
        fm = re.match(r"pub (\w+):", line)
        if fm:
            fields.append(fm.group(1))
    return fields


def state_keys(name):
    """Keys of a state property, for a handler that spreads or returns one."""
    m = re.search("\n      " + name + r":\s*\[?(.*?)" + NEXT_ENTRY, harness, re.S)
    return set(re.findall(r"(\w+):", m.group(1))) if m else set()


def harness_keys(handler):
    m = re.search(re.escape(handler) + r":\s*(.*?)" + NEXT_ENTRY, harness, re.S)
    if not m:
        return None
    body = m.group(1)
    keys = set(re.findall(r"(\w+):", body))
    # A handler that returns or spreads state.foo carries foo's keys too.
    for ref in re.findall(r"state\.(\w+)", body):
        keys |= state_keys(ref)
    return keys


problems = []
for struct, handler in PAIRS.items():
    fields = rust_fields(struct)
    keys = harness_keys(handler)
    if fields is None:
        problems.append(f"  {struct}: no such struct in Rust any more")
        continue
    if keys is None:
        problems.append(f"  {struct}: the harness has no '{handler}' handler")
        continue
    missing = [f for f in fields if f not in keys]
    if missing:
        problems.append(
            f"  {struct} -> {handler}(): harness never returns {', '.join(missing)}"
        )

if problems:
    print("Harness mocks have drifted from the Rust types:", file=sys.stderr)
    print("\n".join(problems), file=sys.stderr)
    print(
        "\nThe UI is being tested against a shape the app does not produce.\n"
        "Update tests/ui-harness.html, or PAIRS here if a type moved.",
        file=sys.stderr,
    )
    sys.exit(1)

print(f"All {len(PAIRS)} mocked types match their Rust definitions.")
CHECK
