#!/usr/bin/env python3
import json
from pathlib import Path
import sys


REQUIRED_SPEC_DIRS = [
    "specs/contracts/gx.resource.v1",
    "specs/contracts/gx.resolver.result.v1",
    "specs/contracts/gx.operation.descriptor.v1",
    "specs/contracts/gx.flow.run.v1",
    "specs/contracts/gx.evidence.v1",
    "specs/contracts/gx.view.v1",
    "specs/profiles/gx.observability.playbook.v1",
]

REQUIRED_CATALOG_FILES = [
    "catalog/core/contracts/index.json",
    "catalog/core/resolvers/index.json",
    "catalog/core/ops/index.json",
    "catalog/core/views/index.json",
    "catalog/core/flow-templates/index.json",
]


def load_json(path: Path):
    with path.open("r", encoding="utf-8") as fh:
        return json.load(fh)


def ensure_file(path: Path):
    if not path.is_file():
        raise SystemExit(f"missing required file: {path}")


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent

    for rel in REQUIRED_SPEC_DIRS:
        spec_dir = repo_root / rel
        if not spec_dir.is_dir():
            raise SystemExit(f"missing required spec directory: {spec_dir}")
        ensure_file(spec_dir / "README.md")
        ensure_file(spec_dir / "manifest.json")
        ensure_file(spec_dir / "schema.json")
        example_dir = spec_dir / "examples"
        if not example_dir.is_dir() or not any(example_dir.iterdir()):
            raise SystemExit(f"missing examples in {example_dir}")
        manifest = load_json(spec_dir / "manifest.json")
        schema_ref = manifest.get("schema_file")
        if schema_ref != "schema.json":
            raise SystemExit(f"unexpected schema_file in {spec_dir / 'manifest.json'}: {schema_ref}")
        load_json(spec_dir / "schema.json")
        for example in manifest.get("example_files", []):
            ensure_file(spec_dir / example)
            load_json(spec_dir / example)

    for rel in REQUIRED_CATALOG_FILES:
        catalog_file = repo_root / rel
        ensure_file(catalog_file)
        catalog = load_json(catalog_file)
        entries = catalog.get("entries")
        if not isinstance(entries, list) or not entries:
            raise SystemExit(f"catalog has no entries: {catalog_file}")

    docs = [
        repo_root / "docs/specs-overview.md",
        repo_root / "docs/core-catalog.md",
        repo_root / "docs/authoring-profile-observability.md",
    ]
    for doc in docs:
        ensure_file(doc)

    print("spec and catalog checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
