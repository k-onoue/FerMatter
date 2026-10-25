#!/usr/bin/env python3
"""Publish a successful main-branch Ubuntu build and its signed APT snapshot."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess

SOURCE = "k-onoue/FerMatter"
PUBLIC = "FerMatter/fermatter.github.io"
TAG = "ubuntu-apt"
ROOT = Path(__file__).resolve().parents[1]


def gh(*arguments):
    return subprocess.check_output(["gh", *arguments], text=True, cwd=ROOT)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_id", type=int)
    parser.add_argument("output", type=Path)
    parser.add_argument("--signing-home", type=Path, required=True)
    options = parser.parse_args()
    spec = importlib.util.spec_from_file_location("apt_builder", ROOT / "scripts/apt-repository.py")
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    config = json.loads((ROOT / "desktop-app/packaging/ubuntu.json").read_text())
    run = json.loads(gh("api", f"repos/{SOURCE}/actions/runs/{options.run_id}"))
    commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    if run["head_branch"] != "main" or run["head_sha"] != commit or run["conclusion"] != "success" or run["path"] != ".github/workflows/ubuntu.yml":
        raise ValueError("Use a successful Ubuntu workflow from the checked-out main commit.")
    if gh("api", f"repos/{SOURCE}/git/ref/heads/main", "--jq", ".object.sha").strip() != commit:
        raise ValueError("Check out current main before publishing.")
    output = options.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    artifacts = output / "artifact"
    gh("run", "download", str(options.run_id), "--repo", SOURCE,
       "--name", f"ubuntu-amd64-{commit}", "--dir", str(artifacts))
    info = json.loads((artifacts / "build-info.json").read_text())
    if info.get("commit") != commit or any(info.get(k) != config[k] for k in ("package", "version", "architecture")):
        raise ValueError("Build provenance differs from the reviewed configuration.")
    name = f"fermatter_{config['version']}_amd64.deb"
    package = artifacts / name
    control = builder.deb_control(package)
    if info.get("file") != name or builder.apt_stage.digest(package) != info.get("sha256") or control.get("X-FerMatter-Commit") != commit or any(control.get(k) != config[v] for k, v in (("Package", "package"), ("Version", "version"), ("Architecture", "architecture"))):
        raise ValueError("Package checksum or source provenance mismatch.")
    releases = json.loads(gh("api", "--paginate", "--slurp", f"repos/{PUBLIC}/releases"))
    previous_release = next((r for page in releases for r in page if r["tag_name"] == TAG), None)
    previous = None
    if previous_release:
        if not any(asset["name"] == "apt-repository.tar.gz" for asset in previous_release["assets"]):
            raise ValueError("Existing APT release has no signed snapshot; restore it before publishing.")
        previous_dir = output / "previous"
        gh("release", "download", TAG, "--repo", PUBLIC, "--pattern", "apt-repository.tar.gz", "--dir", str(previous_dir))
        previous = previous_dir / "apt-repository.tar.gz"
    archive = builder.build(package, output / "snapshot", options.signing_home.resolve(), config["signingFingerprint"], previous)
    if not previous_release:
        gh("release", "create", TAG, "--repo", PUBLIC, "--prerelease", "--title", "Experimental Ubuntu APT repository",
           "--notes", "Signed APT snapshots and experimental Ubuntu 24.04 amd64 packages. Installation: https://fermatter.github.io/download/#ubuntu")
    published_names = {asset["name"] for asset in previous_release["assets"]} if previous_release else set()
    if name in published_names:
        existing = output / "existing"
        gh("release", "download", TAG, "--repo", PUBLIC, "--pattern", name, "--dir", str(existing))
        if builder.apt_stage.digest(existing / name) != builder.apt_stage.digest(package):
            raise ValueError("Refusing to replace an immutable published Debian package.")
    else:
        gh("release", "upload", TAG, "--repo", PUBLIC, str(package))
    gh("release", "upload", TAG, "--repo", PUBLIC, str(archive), "--clobber")
    gh("variable", "set", "FERMATTER_APT_AVAILABLE", "--repo", PUBLIC, "--body", "true")
    gh("workflow", "run", "pages.yml", "--repo", PUBLIC, "--ref", "main")
    print(json.dumps({"commit": commit, "version": config["version"], "sha256": info["sha256"], "repository": "https://fermatter.github.io/apt/"}))


if __name__ == "__main__":
    main()
