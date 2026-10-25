#!/usr/bin/env python3
"""Verify a signed, package-only APT archive before staging it for Pages."""
import argparse
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
import gzip
import hashlib
import io
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tarfile
import tempfile

PACKAGE_LIMIT = 100 * 1024 * 1024
INDEX_LIMIT = 16 * 1024 * 1024
REPOSITORY_LIMIT = 1024 * 1024 * 1024
INDEX = Path("dists/experimental/main/binary-amd64")
POOL = "pool/main/f/fermatter/"
SOURCES = """Types: deb
URIs: https://fermatter.github.io/apt
Suites: experimental
Components: main
Architectures: amd64
Signed-By: /etc/apt/keyrings/fermatter.gpg
"""


def fields(text):
    result = {}
    key = None
    for line in text.splitlines():
        if line.startswith((" ", "\t")) and key:
            result[key] += "\n" + line
        elif line:
            key, separator, value = line.partition(":")
            if not separator or not re.fullmatch(r"[A-Za-z0-9-]+", key) or key in result:
                raise ValueError("Invalid or duplicate Debian field.")
            result[key] = value.strip()
    return result


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def allowed(name):
    if name in ("fermatter.gpg", "fermatter.sources"):
        return True
    if re.fullmatch(r"pool/main/f/fermatter/fermatter_[0-9A-Za-z.+~_-]+_amd64\.deb", name):
        return True
    if name in ("dists/experimental/Release", "dists/experimental/InRelease", "dists/experimental/Release.gpg"):
        return True
    prefix = INDEX.as_posix() + "/"
    return name.startswith(prefix) and (
        name[len(prefix):] in ("Packages", "Packages.gz")
        or re.fullmatch(r"by-hash/SHA256/[0-9a-f]{64}", name[len(prefix):]) is not None
    )


def verify(root, fingerprint):
    if not re.fullmatch(r"[A-F0-9]{40}", fingerprint):
        raise ValueError("Invalid pinned signing fingerprint.")
    key = root / "fermatter.gpg"
    with tempfile.TemporaryDirectory(prefix="fermatter-apt-verify-") as temporary:
        home = Path(temporary)
        public = subprocess.check_output(
            ["gpg", "--batch", "--homedir", str(home), "--with-colons", "--show-keys", str(key)],
            text=True, stderr=subprocess.PIPE,
        )
        fingerprints = [line.split(":")[9] for line in public.splitlines() if line.startswith("fpr:")]
        if fingerprints != [fingerprint]:
            raise ValueError("APT signing key does not match the pinned fingerprint.")
        decoded = home / "Release"
        subprocess.run(
            ["gpg", "--batch", "--homedir", str(home), "--import", str(key)],
            check=True, capture_output=True,
        )
        subprocess.run(
            ["gpg", "--batch", "--homedir", str(home), "--output", str(decoded),
             "--decrypt", str(root / "dists/experimental/InRelease")],
            check=True, capture_output=True,
        )
        release = root / "dists/experimental/Release"
        if decoded.read_bytes() != release.read_bytes():
            raise ValueError("Release differs from its signed contents.")
        subprocess.run(
            ["gpgv", "--homedir", str(home), "--keyring", str(key.resolve()),
             str(release.with_name("Release.gpg")), str(release)],
            check=True, capture_output=True,
        )
    metadata = fields(release.read_text())
    for name, value in {"Origin": "FerMatter", "Suite": "experimental", "Codename": "experimental",
                        "Architectures": "amd64", "Components": "main", "Acquire-By-Hash": "yes"}.items():
        if metadata.get(name) != value:
            raise ValueError(f"Unexpected APT release field: {name}.")
    if parsedate_to_datetime(metadata["Valid-Until"]) <= datetime.now(timezone.utc):
        raise ValueError("APT metadata has expired.")
    expected = {"main/binary-amd64/Packages", "main/binary-amd64/Packages.gz"}
    for record in metadata["SHA256"].splitlines():
        if not record.strip():
            continue
        checksum, size, name = record.split()
        if name not in expected:
            raise ValueError("Unexpected or duplicate APT index.")
        expected.remove(name)
        file = root / "dists/experimental" / name
        if file.stat().st_size != int(size) or digest(file) != checksum:
            raise ValueError("APT index checksum mismatch.")
        if not (file.parent / "by-hash/SHA256" / checksum).is_file():
            raise ValueError("Missing immutable APT index.")
    if expected:
        raise ValueError("Missing APT index checksums.")
    packages = root / INDEX / "Packages"
    if packages.stat().st_size > INDEX_LIMIT:
        raise ValueError("APT index exceeds its size limit.")
    with gzip.GzipFile(fileobj=io.BytesIO(packages.with_name("Packages.gz").read_bytes())) as stream:
        if stream.read(INDEX_LIMIT + 1) != packages.read_bytes():
            raise ValueError("Compressed and plain APT indexes differ.")
    registered = set()
    for stanza in packages.read_text().strip().split("\n\n"):
        package = fields(stanza)
        if package.get("Package") != "fermatter" or package.get("Architecture") != "amd64":
            raise ValueError("The repository may contain only FerMatter amd64 packages.")
        name = package["Filename"]
        if not name.startswith(POOL) or not allowed(name) or name in registered:
            raise ValueError("Invalid package filename.")
        registered.add(name)
        file = root / name
        if file.stat().st_size != int(package["Size"]) or digest(file) != package["SHA256"]:
            raise ValueError("Debian package checksum mismatch.")
    for file in root.rglob("*"):
        if file.is_file():
            name = file.relative_to(root).as_posix()
            if not allowed(name) or (name.endswith(".deb") and name not in registered):
                raise ValueError("Unexpected file in APT repository.")
            if "/by-hash/SHA256/" in name and digest(file) != file.name:
                raise ValueError("Immutable APT index checksum mismatch.")
    if (root / "fermatter.sources").read_text() != SOURCES:
        raise ValueError("Unexpected APT sources configuration.")
    return registered


def stage(archive, destination, fingerprint):
    destination = Path(destination)
    if (destination / "apt").exists() or (destination / "apt").is_symlink():
        raise ValueError("Refusing to replace an existing staged APT repository.")
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".apt-", dir=destination) as temporary:
        extracted = Path(temporary)
        total = 0
        seen = set()
        with tarfile.open(archive, "r:gz") as source:
            for member in source:
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != "apt":
                    raise ValueError("APT archive path leaves its root.")
                if member.isdir():
                    continue
                name = path.relative_to("apt").as_posix()
                total += member.size
                if not member.isfile() or not allowed(name) or name in seen or member.size < 0 or member.size > PACKAGE_LIMIT or total > REPOSITORY_LIMIT:
                    raise ValueError("Unexpected APT archive entry or size.")
                seen.add(name)
                output = extracted / "apt" / name
                output.parent.mkdir(parents=True, exist_ok=True)
                with source.extractfile(member) as incoming, output.open("xb") as outgoing:
                    shutil.copyfileobj(incoming, outgoing)
        verify(extracted / "apt", fingerprint)
        (extracted / "apt").rename(destination / "apt")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--fingerprint", required=True)
    options = parser.parse_args()
    stage(options.archive, options.destination, options.fingerprint)
    print("Verified and staged the signed FerMatter APT repository.")


if __name__ == "__main__":
    main()
