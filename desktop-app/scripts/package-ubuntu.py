#!/usr/bin/env python3
"""Give the Tauri Debian bundle its stable package name and preview version."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def main():
    desktop = Path(__file__).resolve().parents[1]
    repository = desktop.parent
    config = json.loads((desktop / "packaging/ubuntu.json").read_text())
    packages = list((desktop / "src-tauri/target/release/bundle/deb").glob("*.deb"))
    if len(packages) != 1:
        raise RuntimeError("Expected exactly one Tauri Debian bundle.")
    commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=repository, text=True
    ).strip()
    output = desktop / "build/ubuntu"
    output.mkdir(parents=True, exist_ok=True)
    name = "{package}_{version}_{architecture}.deb".format(**config)
    target = output / name
    with tempfile.TemporaryDirectory(prefix="fermatter-deb-") as temporary:
        stage = Path(temporary) / "package"
        subprocess.run(["dpkg-deb", "--raw-extract", str(packages[0]), str(stage)], check=True)
        control = stage / "DEBIAN/control"
        fields = {}
        for line in control.read_text().splitlines():
            if line.startswith((" ", "\t")):
                fields[key] += "\n" + line
            elif line:
                key, value = line.split(":", 1)
                fields[key] = value.strip()
        if fields["Architecture"] != config["architecture"]:
            raise RuntimeError("Unexpected package architecture.")
        for script in ("preinst", "postinst", "prerm", "postrm", "config"):
            if (stage / "DEBIAN" / script).exists():
                raise RuntimeError("The Ubuntu package must not run installer scripts.")
        fields["Package"] = config["package"]
        fields["Version"] = config["version"]
        fields["X-FerMatter-Commit"] = commit
        dependencies = fields.get("Depends", "")
        fields["Depends"] = (dependencies + ", " if dependencies else "") + "xdg-utils"
        control.write_text("".join(f"{key}: {value}\n" for key, value in fields.items()))
        if not (stage / "usr/bin/fermatter").is_file():
            raise RuntimeError("The package must install /usr/bin/fermatter.")
        subprocess.run(
            ["dpkg-deb", "--build", "--root-owner-group", "-Zgzip", str(stage), str(target)],
            check=True,
        )
    result = {
        **config,
        "commit": commit,
        "file": name,
        "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
        "control": fields,
    }
    (output / "build-info.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result[key] for key in ("file", "commit", "sha256")}))


if __name__ == "__main__":
    main()
