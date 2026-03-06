#!/usr/bin/env python3

import argparse
import json
import subprocess
import sys
from collections import defaultdict, deque
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent


def load_metadata():
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def load_workspace_version():
    cargo_toml = REPO_ROOT / "Cargo.toml"
    if sys.version_info >= (3, 11):
        import tomllib
    else:
        import tomli as tomllib

    with cargo_toml.open("rb") as handle:
        data = tomllib.load(handle)

    workspace_package = data.get("workspace", {}).get("package", {})
    version = workspace_package.get("version")
    if not version:
        sys.stderr.write("workspace.package.version is not set in Cargo.toml\n")
        sys.exit(1)
    return version


def publishable_packages(metadata):
    workspace_members = set(metadata["workspace_members"])
    packages = {pkg["id"]: pkg for pkg in metadata["packages"] if pkg["id"] in workspace_members}

    publishable = {}
    for pkg_id, pkg in packages.items():
        if pkg.get("publish") == []:
            continue
        publishable[pkg_id] = pkg

    edges = defaultdict(set)
    indegree = {pkg_id: 0 for pkg_id in publishable}
    for pkg_id, pkg in publishable.items():
        for dep in pkg.get("dependencies", []):
            dep_id = dep.get("path")
            if dep_id is None:
                continue
        for dep_pkg_id, dep_pkg in publishable.items():
            if dep_pkg_id == pkg_id:
                continue
            dep_manifest_dir = dep_pkg["manifest_path"].rsplit("/", 1)[0]
            for dep in pkg.get("dependencies", []):
                dep_path = dep.get("path")
                if dep_path == dep_manifest_dir:
                    if pkg_id not in edges[dep_pkg_id]:
                        edges[dep_pkg_id].add(pkg_id)
                        indegree[pkg_id] += 1

    queue = deque(sorted(pkg_id for pkg_id, count in indegree.items() if count == 0))
    ordered = []
    while queue:
        pkg_id = queue.popleft()
        ordered.append(publishable[pkg_id])
        for next_pkg_id in sorted(edges[pkg_id]):
            indegree[next_pkg_id] -= 1
            if indegree[next_pkg_id] == 0:
                queue.append(next_pkg_id)

    if len(ordered) != len(publishable):
        sys.stderr.write("failed to derive publish order for workspace crates\n")
        sys.exit(1)

    return ordered


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--format", choices=["names", "details", "workspace-version"], default="names")
    args = parser.parse_args()

    if args.format == "workspace-version":
        print(load_workspace_version())
        return

    metadata = load_metadata()
    crates = publishable_packages(metadata)

    for pkg in crates:
        if args.format == "names":
            print(pkg["name"])
        else:
            print(f'{pkg["name"]}\t{pkg["manifest_path"]}')


if __name__ == "__main__":
    main()
