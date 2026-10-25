#!/usr/bin/env python3
"""Build a signed APT snapshot without committing packages or private keys."""
import argparse
from datetime import datetime, timedelta, timezone
from email.utils import format_datetime
import gzip
import importlib.util
import io
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile

spec = importlib.util.spec_from_file_location("apt_stage", Path(__file__).with_name("stage-apt-repository.py"))
apt_stage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(apt_stage)
ROOT = Path(__file__).resolve().parents[1]


def deb_control(package):
    if package.stat().st_size > apt_stage.PACKAGE_LIMIT:
        raise ValueError("Debian package exceeds its size limit.")
    with package.open("rb") as stream:
        if stream.read(8) != b"!<arch>\n":
            raise ValueError("Not a Debian ar archive.")
        while header := stream.read(60):
            if len(header) != 60 or header[58:] != b"`\n":
                raise ValueError("Invalid Debian ar header.")
            name = header[:16].decode("ascii").strip().rstrip("/")
            size = int(header[48:58])
            if size < 0 or size > apt_stage.PACKAGE_LIMIT:
                raise ValueError("Invalid Debian ar member size.")
            data = stream.read(size)
            if len(data) != size:
                raise ValueError("Truncated Debian ar member.")
            if size % 2:
                stream.read(1)
            if name in ("control.tar.gz", "control.tar.xz", "control.tar"):
                with tarfile.open(fileobj=io.BytesIO(data), mode="r:*") as control:
                    members = [m for m in control if m.name.lstrip("./") == "control"]
                    if len(members) != 1 or not members[0].isfile() or members[0].size > 256 * 1024:
                        raise ValueError("Invalid Debian control file.")
                    return apt_stage.fields(control.extractfile(members[0]).read().decode())
    raise ValueError("Missing Debian control archive.")


def build(package, output, signing_home, fingerprint, previous=None):
    package = Path(package)
    output = Path(output)
    control = deb_control(package)
    version = control.get("Version", "")
    if control.get("Package") != "fermatter" or control.get("Architecture") != "amd64" or not re.fullmatch(r"[0-9][0-9A-Za-z.+~_-]*", version):
        raise ValueError("Expected a versioned FerMatter amd64 package.")
    if not re.fullmatch(r"[a-f0-9]{40}", control.get("X-FerMatter-Commit", "")):
        raise ValueError("Missing source commit provenance.")
    if output.exists():
        raise ValueError("Choose a new output directory to preserve previous snapshots.")
    output.mkdir(parents=True)
    if previous:
        apt_stage.stage(previous, output, fingerprint)
    repository = output / "apt"
    pool = repository / apt_stage.POOL
    pool.mkdir(parents=True, exist_ok=True)
    filename = f"fermatter_{version}_amd64.deb"
    target = pool / filename
    if target.exists() and apt_stage.digest(target) != apt_stage.digest(package):
        raise ValueError("Refusing to replace an already published package version.")
    if not target.exists():
        shutil.copyfile(package, target)
    records = []
    for file in sorted(pool.glob("*.deb")):
        record = deb_control(file)
        if record.get("Package") != "fermatter" or record.get("Architecture") != "amd64":
            raise ValueError("Unexpected package in previous snapshot.")
        for field in ("Filename", "Size", "MD5sum", "SHA1", "SHA256", "SHA512"):
            record.pop(field, None)
        record["Filename"] = file.relative_to(repository).as_posix()
        record["Size"] = str(file.stat().st_size)
        record["SHA256"] = apt_stage.digest(file)
        records.append("".join(f"{key}: {value}\n" for key, value in record.items()))
    indexes = repository / apt_stage.INDEX
    indexes.mkdir(parents=True, exist_ok=True)
    contents = ("\n".join(records) + "\n").encode()
    (indexes / "Packages").write_bytes(contents)
    (indexes / "Packages.gz").write_bytes(gzip.compress(contents, mtime=0))
    checksums = []
    for name in ("Packages", "Packages.gz"):
        file = indexes / name
        checksum = apt_stage.digest(file)
        immutable = indexes / "by-hash/SHA256" / checksum
        immutable.parent.mkdir(parents=True, exist_ok=True)
        immutable.write_bytes(file.read_bytes())
        checksums.append(f" {checksum} {file.stat().st_size} main/binary-amd64/{name}")
    now = datetime.now(timezone.utc)
    release = repository / "dists/experimental/Release"
    release.write_text(
        "Origin: FerMatter\nLabel: FerMatter experimental Ubuntu packages\n"
        "Suite: experimental\nCodename: experimental\n"
        f"Date: {format_datetime(now, usegmt=True)}\n"
        f"Valid-Until: {format_datetime(now + timedelta(days=365), usegmt=True)}\n"
        "Architectures: amd64\nComponents: main\nAcquire-By-Hash: yes\n"
        "Description: Experimental FerMatter packages for Ubuntu 24.04 amd64\n"
        "SHA256:\n" + "\n".join(checksums) + "\n"
    )
    gpg = ["gpg", "--homedir", str(signing_home), "--batch", "--yes", "--local-user", fingerprint]
    (repository / "fermatter.gpg").write_bytes(subprocess.check_output(gpg + ["--export", fingerprint]))
    (repository / "fermatter.sources").write_text(apt_stage.SOURCES)
    subprocess.run(gpg + ["--armor", "--clearsign", "--output", str(release.with_name("InRelease")), str(release)], check=True)
    subprocess.run(gpg + ["--armor", "--detach-sign", "--output", str(release.with_name("Release.gpg")), str(release)], check=True)
    apt_stage.verify(repository, fingerprint)
    archive = output / "apt-repository.tar.gz"
    with tarfile.open(archive, "w:gz") as destination:
        destination.add(repository, arcname="apt")
    return archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--signing-home", type=Path, required=True)
    parser.add_argument("--previous", type=Path)
    options = parser.parse_args()
    config = json.loads((ROOT / "desktop-app/packaging/ubuntu.json").read_text())
    control = deb_control(options.package)
    if control.get("Version") != config["version"]:
        raise ValueError("Package version differs from the reviewed Ubuntu configuration.")
    archive = build(options.package, options.output, options.signing_home, config["signingFingerprint"], options.previous)
    print(json.dumps({"archive": str(archive), "sha256": apt_stage.digest(archive)}))


if __name__ == "__main__":
    main()
