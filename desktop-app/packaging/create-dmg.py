"""Create and inspect the Finder layout without automating Finder."""

import argparse
from pathlib import Path

import dmgbuild
from ds_store import DSStore


def build(source, output, volume, background):
    settings = {
        "format": "UDZO",
        "filesystem": "HFS+",
        "files": [
            str(source / name)
            for name in ("FerMatter.app", "README.txt", "LICENSE", "NOTICE")
        ],
        "symlinks": {"Applications": "/Applications"},
        "background": str(background),
        "window_rect": ((200, 200), (640, 280)),
        "default_view": "icon-view",
        "icon_size": 96,
        "text_size": 14,
        "icon_locations": {"FerMatter.app": (140, 120), "Applications": (500, 120)},
        "hide": ["README.txt", "LICENSE", "NOTICE"],
        "show_toolbar": False,
        "show_status_bar": False,
        "show_pathbar": False,
        "show_sidebar": False,
    }
    dmgbuild.build_dmg(str(output), volume, settings=settings)


def verify(volume, background):
    expected = background.read_bytes()
    if (volume / ".background.tiff").read_bytes() != expected:
        raise ValueError("DMG is missing the installation arrow background")
    with DSStore.open(str(volume / ".DS_Store"), "r") as store:
        if tuple(store["FerMatter.app"]["Iloc"]) != (140, 120):
            raise ValueError("Unexpected FerMatter icon position")
        if tuple(store["Applications"]["Iloc"]) != (500, 120):
            raise ValueError("Unexpected Applications icon position")
        view = store["."]["icvp"]
        if view["backgroundType"] != 2 or not view.get("backgroundImageAlias"):
            raise ValueError("Finder is not configured to show the arrow background")
        if view["iconSize"] != 96:
            raise ValueError("Unexpected Finder icon size")
    print("Verified Finder installation arrow and app → Applications positions.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    create = commands.add_parser("build")
    create.add_argument("source", type=Path)
    create.add_argument("output", type=Path)
    create.add_argument("volume")
    create.add_argument("background", type=Path)
    inspect = commands.add_parser("verify")
    inspect.add_argument("volume", type=Path)
    inspect.add_argument("background", type=Path)
    args = parser.parse_args()
    if args.command == "build":
        build(args.source, args.output, args.volume, args.background)
    else:
        verify(args.volume, args.background)
