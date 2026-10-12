#!/usr/bin/env python3
"""Check synchronized crate versions and optional vX.Y.Z release tags."""

import argparse
import json
from pathlib import Path
import re
import subprocess
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", help="Release tag, for example v0.1.0")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    version = manifest["workspace"]["package"]["version"]
    errors = []
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", version):
        errors.append(f"Invalid release version: {version}")
    if args.tag is not None and args.tag != f"v{version}":
        errors.append(f"Tag {args.tag!r} must match v{version}")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
        cwd=root, text=True,
    ))
    members = set(metadata["workspace_members"])
    packages = {p["name"]: p for p in metadata["packages"] if p["id"] in members}
    publishable = {name: p for name, p in packages.items() if p["publish"] != []}
    for name, package in publishable.items():
        if not (name == "bevy_wings" or name.startswith("bevy_wings_")) or "-" in name:
            errors.append(f"{name} must use the bevy_wings package family with underscores")
        if package["version"] != version:
            errors.append(f"{name} version {package['version']} differs from {version}")
        for field in ("description", "license", "repository", "rust_version"):
            if not package.get(field):
                errors.append(f"{name} is missing {field}")
        for dep in package["dependencies"]:
            if dep["name"] in packages:
                if dep["name"] not in publishable:
                    errors.append(f"{name} depends on unpublished {dep['name']}")
                if dep["req"] not in (f"^{version}", f"={version}"):
                    errors.append(f"{name} dependency {dep['name']} must use version {version}, got {dep['req']}")
    if errors:
        parser.exit(1, "\n".join(errors) + "\n")
    print(f"Validated {len(publishable)} publishable crates at {version}")


if __name__ == "__main__":
    main()
