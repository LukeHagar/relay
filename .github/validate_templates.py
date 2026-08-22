#!/usr/bin/env python3
"""CI guard: every bundled template fixture must be valid JSON with required fields."""
import json, sys, pathlib

root = pathlib.Path(__file__).resolve().parent.parent / "templates"
failures = []
files = sorted(root.rglob("*.json"))
if not files:
    failures.append("no template files found")
for path in files:
    try:
        t = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        failures.append(f"{path}: invalid JSON: {e}")
        continue
    for field in ("v", "provider", "name", "request"):
        if field not in t:
            failures.append(f"{path}: missing '{field}'")
    req = t.get("request", {})
    if "method" not in req or "body" not in req:
        failures.append(f"{path}: request needs method and body")

if failures:
    print("\n".join(failures))
    sys.exit(1)
print(f"validated {len(files)} template fixtures")
