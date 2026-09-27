#!/usr/bin/env python3
"""Build the downloadable demo from an explicit list of public source files."""
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DEMOS = {
    'demo-tikz': ('README.md', 'main.tex', 'latexmkrc'),
    'demo-cetz': ('README.md', 'main.typ'),
    'demo-paper': ('README.md', 'main.tex', 'latexmkrc', 'sections/model.tex',
                   'sections/fitting.tex', 'sections/geometry.tex', 'sections/summary.tex'),
    'demo-typst': ('README.md', 'main.typ', 'symbols.typ', 'sections/model.typ', 'sections/fitting.typ'),
}


def build():
    for demo_name, files in DEMOS.items():
        demo = ROOT / 'examples' / demo_name
        output = ROOT / 'docs/downloads' / (demo_name + '.zip')
        output.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
            for name in files:
                source = demo / name
                if source.is_symlink() or not source.resolve().is_relative_to(demo.resolve()):
                    raise ValueError(f'Demo source must be a regular file inside the demo folder: {name}')
                entry = zipfile.ZipInfo(demo_name + '/' + name, date_time=(2026, 1, 1, 0, 0, 0))
                entry.compress_type = zipfile.ZIP_DEFLATED
                entry.external_attr = 0o100644 << 16
                archive.writestr(entry, source.read_bytes())
        print(f'Prepared {output.relative_to(ROOT)} ({len(files)} source files).')


if __name__ == '__main__':
    build()
