#!/usr/bin/env bash
#
# Catches string literals accidentally broken across lines.
#
# Editing these files through a script that mishandles a backslash turns
# "a\nb" into a real newline inside the quotes. In JavaScript that is a
# syntax error and the whole frontend fails to load - the harness still
# renders, so it looks fine until nothing responds. In Rust a string may
# legally span lines, so it compiles and the mistake ships. Both happened
# here more than once, and CI only ever caught the JS.
#
# Deliberately dumb: it counts unescaped quotes per line rather than
# parsing. Raw strings are skipped, and a line that opens a string on
# purpose can be allowlisted.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'CHECK'
import re
import sys
from pathlib import Path

# Lines that legitimately open a string that continues on the next line,
# or that carry an odd number of quotes for a good reason.
ALLOW = (
    re.compile(r'replace\(/"/g'),      # a regex matching a quote
    re.compile(r'": "Backslash"'),     # the key-name map
    re.compile(r'let tsv = "level'),   # a deliberate multi-line TSV fixture
    re.compile(r"replace\('\"'"),        # escaping a quote for HTML
)

# rglob, not glob: system_ocr.rs became system_ocr/ with a file per
# platform, and a non-recursive glob silently stopped checking all three.
# A guard that quietly covers less than it did is worse than none.
files = (
    sorted(Path("src").rglob("*.js"))
    + sorted(Path("tests").rglob("*.js"))
    + sorted(Path("src-tauri/src").rglob("*.rs"))
    + sorted(Path("src-tauri/tests").rglob("*.rs"))
)

failures = []

for path in files:
    in_raw = False
    in_string = False
    for n, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
        # Raw strings may span lines on purpose.
        if 'r#"' in line:
            in_raw = '"#' not in line.split('r#"', 1)[1]
            continue
        if in_raw:
            if '"#' in line:
                in_raw = False
            continue

        # Drop escaped quotes, then a comment only when it starts the line
        # (so a // inside a URL is not mistaken for one).
        stripped = line.replace('\\"', "")
        if re.match(r"^\s*//", stripped):
            continue
        if stripped.count('"') % 2 == 0:
            continue

        if in_string:
            in_string = False  # this line closes the one before it
        else:
            in_string = True
            if not any(a.search(line) for a in ALLOW):
                failures.append(f"  {path.as_posix()}:{n}  {line.strip()[:70]}")

if failures:
    print("String literals broken across lines:", file=sys.stderr)
    print("\n".join(failures), file=sys.stderr)
    print(
        "\nAn odd number of quotes usually means a \\n became a real newline.\n"
        "Write the escape, or add the line to ALLOW in this script.",
        file=sys.stderr,
    )
    sys.exit(1)

print(f"No string literals are broken across lines ({len(files)} files).")
CHECK
