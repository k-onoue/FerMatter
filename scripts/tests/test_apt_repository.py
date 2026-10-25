import gzip
import importlib.util
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("apt_repository", Path(__file__).resolve().parents[1] / "apt-repository.py")
repository = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repository)


def package(path, version="1.0~preview.1", name="fermatter", description="Synthetic fixture"):
    control = (
        f"Package: {name}\nVersion: {version}\nArchitecture: amd64\n"
        f"Maintainer: FerMatter\nDescription: {description}\n"
        f"X-FerMatter-Commit: {'a' * 40}\n"
    ).encode()
    archive = io.BytesIO()
    with tarfile.open(fileobj=archive, mode="w") as source:
        info = tarfile.TarInfo("./control")
        info.size = len(control)
        source.addfile(info, io.BytesIO(control))
    members = [("debian-binary", b"2.0\n"), ("control.tar.gz", gzip.compress(archive.getvalue()))]
    with path.open("wb") as output:
        output.write(b"!<arch>\n")
        for name, data in members:
            header = f"{name + '/':<16}{0:<12}{0:<6}{0:<6}{'100644':<8}{len(data):<10}`\n".encode()
            output.write(header + data + (b"\n" if len(data) % 2 else b""))
    return path


class RepositoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.keys = tempfile.TemporaryDirectory(prefix="fermatter-test-key-")
        cls.signing = Path(cls.keys.name)
        subprocess.run(
            ["gpg", "--homedir", str(cls.signing), "--batch", "--pinentry-mode", "loopback",
             "--passphrase", "", "--quick-generate-key", "FerMatter repository test", "ed25519", "sign", "1d"],
            check=True, capture_output=True,
        )
        keys = subprocess.check_output(
            ["gpg", "--homedir", str(cls.signing), "--batch", "--with-colons", "--list-keys"],
            text=True, stderr=subprocess.PIPE,
        )
        cls.fingerprint = next(line.split(":")[9] for line in keys.splitlines() if line.startswith("fpr:"))

    @classmethod
    def tearDownClass(cls):
        subprocess.run(["gpgconf", "--homedir", str(cls.signing), "--kill", "all"], check=True)
        cls.keys.cleanup()

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="fermatter-apt-test-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def build(self, file, name="repository", previous=None):
        return repository.build(file, self.root / name, self.signing, self.fingerprint, previous)

    def test_signed_snapshot_stages_with_scoped_key_and_source_configuration(self):
        archive = self.build(package(self.root / "fixture.deb"))
        destination = self.root / "site"
        repository.apt_stage.stage(archive, destination, self.fingerprint)
        apt = destination / "apt"
        records = repository.apt_stage.verify(apt, self.fingerprint)
        self.assertEqual(records, {"pool/main/f/fermatter/fermatter_1.0~preview.1_amd64.deb"})
        self.assertIn("Signed-By: /etc/apt/keyrings/fermatter.gpg", (apt / "fermatter.sources").read_text())

    def test_previous_versions_and_immutable_indexes_survive_an_update(self):
        first = self.build(package(self.root / "one.deb"))
        original = self.root / "repository/apt"
        old_indexes = {p.name for p in (original / repository.apt_stage.INDEX / "by-hash/SHA256").iterdir()}
        second = self.build(package(self.root / "two.deb", version="1.0~preview.2"), "next", first)
        destination = self.root / "site"
        repository.apt_stage.stage(second, destination, self.fingerprint)
        current = destination / "apt"
        self.assertEqual(len(repository.apt_stage.verify(current, self.fingerprint)), 2)
        self.assertTrue(old_indexes.issubset({p.name for p in (current / repository.apt_stage.INDEX / "by-hash/SHA256").iterdir()}))

    def test_reusing_a_version_for_different_bytes_is_rejected(self):
        first = self.build(package(self.root / "one.deb"))
        with self.assertRaisesRegex(ValueError, "already published"):
            self.build(package(self.root / "different.deb", description="Changed"), "next", first)

    def test_tampered_index_is_not_published(self):
        archive = self.build(package(self.root / "fixture.deb"))
        source = self.root / "tampered"
        repository.apt_stage.stage(archive, source, self.fingerprint)
        (source / "apt" / repository.apt_stage.INDEX / "Packages").write_text("Modified")
        tampered = self.root / "bad.tar.gz"
        with tarfile.open(tampered, "w:gz") as output:
            output.add(source / "apt", arcname="apt")
        with self.assertRaisesRegex(ValueError, "checksum"):
            repository.apt_stage.stage(tampered, self.root / "site", self.fingerprint)
        self.assertFalse((self.root / "site/apt").exists())

    def test_unexpected_signer_and_unrelated_packages_are_rejected(self):
        archive = self.build(package(self.root / "fixture.deb"))
        with self.assertRaisesRegex(ValueError, "fingerprint"):
            repository.apt_stage.stage(archive, self.root / "site", "0" * 40)
        with self.assertRaisesRegex(ValueError, "FerMatter"):
            self.build(package(self.root / "system.deb", name="systemd"), "unrelated")

    def test_archive_traversal_and_links_are_rejected(self):
        for name, link in (("apt/../../escape", False), ("apt/fermatter.gpg", True)):
            with self.subTest(name=name):
                archive = self.root / "hostile.tar.gz"
                with tarfile.open(archive, "w:gz") as output:
                    member = tarfile.TarInfo(name)
                    if link:
                        member.type = tarfile.SYMTYPE
                        member.linkname = "/etc/passwd"
                    else:
                        member.size = 1
                    output.addfile(member, None if link else io.BytesIO(b"x"))
                with self.assertRaises(ValueError):
                    repository.apt_stage.stage(archive, self.root / "site", self.fingerprint)
                self.assertFalse((self.root / "escape").exists())


if __name__ == "__main__":
    unittest.main()
