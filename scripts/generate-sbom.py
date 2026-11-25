#!/usr/bin/env python3
"""Generate a small CycloneDX 1.5 SBOM from Cargo.lock and pubspec.lock."""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSION = (ROOT / "VERSION").read_text(encoding="utf-8").strip()


def cargo_components() -> list[dict]:
    text = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
    components = []
    for block in text.split("[[package]]")[1:]:
        name = re.search(r'^name = "(.+)"$', block, re.MULTILINE)
        version = re.search(r'^version = "(.+)"$', block, re.MULTILINE)
        if not name or not version:
            continue
        purl = f"pkg:cargo/{name.group(1)}@{version.group(1)}"
        components.append(
            {
                "type": "library",
                "name": name.group(1),
                "version": version.group(1),
                "purl": purl,
                "licenses": [],
            }
        )
    return components


def pub_components() -> list[dict]:
    lock = pathlib.Path(ROOT / "apps/desktop/pubspec.lock")
    if not lock.exists():
        return []
    components = []
    in_packages = False
    current: str | None = None
    version = "0"
    source = "hosted"
    for raw in lock.read_text(encoding="utf-8").splitlines():
        if raw == "packages:":
            in_packages = True
            continue
        if not in_packages:
            continue
        if raw and not raw.startswith(" "):
            break
        match = re.match(r"^  ([A-Za-z0-9_-]+):$", raw)
        if match:
            if current:
                components.append(_pub_component(current, version, source))
            current = match.group(1)
            version = "0"
            source = "hosted"
            continue
        if current is None:
            continue
        stripped = raw.strip()
        if stripped.startswith("source:"):
            source = stripped.split(":", 1)[1].strip()
        elif stripped.startswith("version:"):
            version = stripped.split(":", 1)[1].strip().strip('"')
    if current:
        components.append(_pub_component(current, version, source))
    return components


def _pub_component(name: str, version: str, source: str) -> dict:
    if source == "hosted":
        purl = f"pkg:pub/{name}@{version}"
    else:
        purl = f"pkg:generic/{name}@{version}"
    return {
        "type": "library",
        "name": name,
        "version": version,
        "purl": purl,
        "licenses": [],
    }


def main() -> int:
    components = [
        {
            "type": "application",
            "name": "KONCET",
            "version": VERSION,
            "purl": f"pkg:generic/koncet@{VERSION}",
            "licenses": [{"license": {"id": "GPL-3.0-or-later"}}],
        },
        *cargo_components(),
        *pub_components(),
    ]
    sbom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": "KONCET",
                "version": VERSION,
                "licenses": [{"license": {"id": "GPL-3.0-or-later"}}],
            }
        },
        "components": components,
    }
    output = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "target/package/sbom.cdx.json")
    pathlib.Path(output).write_text(json.dumps(sbom, indent=2) + "\n", encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
