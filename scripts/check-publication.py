#!/usr/bin/env python3
"""Check publication candidates, local site links, and the sample ZIP. No uploads."""
from html.parser import HTMLParser
import importlib.util
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit
import zipfile

ROOT = Path(__file__).resolve().parents[1]
errors = []


def git(*args):
    return subprocess.check_output(['git', '-C', str(ROOT), *args])


# Include tracked files even when ignored, and new files that `git add .` would add.
files = sorted(set(git('ls-files', '-z', '--cached', '--others', '--exclude-standard').decode().strip('\0').split('\0')) - {''})
for name in files:
    path = PurePosixPath(name)
    forbidden = path.parts[0] in {'paper', '.private'} or any(part in {'build', '.build', 'node_modules', '__pycache__', 'dist'} for part in path.parts)
    forbidden |= path.name.startswith('.env') and path.name != '.env.example'
    forbidden |= path.suffix.lower() in {'.p12', '.p8', '.mobileprovision', '.dmg'}
    if forbidden:
        errors.append(f'Private or generated file is a publication candidate: {name}')
    local = ROOT / name
    if local.is_symlink():
        errors.append(f'Review and remove symlink before publishing: {name}')
        continue
    if not local.is_file():
        continue
    try:
        content = local.read_text(encoding='utf-8')
    except UnicodeDecodeError:
        continue
    if re.search(r'/(?:Users|home)/[A-Za-z0-9_.-]+/', content):
        errors.append(f'Personal absolute path found: {name}')
    if re.search(r'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----', content):
        errors.append(f'Private key found: {name}')
    if re.search(r'(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})', content):
        errors.append(f'Credential-like content found: {name}')


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.references, self.ids = [], set()

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get('id'):
            self.ids.add(attrs['id'])
        for key in ('href', 'src'):
            if attrs.get(key):
                self.references.append(attrs[key])


site = ROOT / 'docs'
for html in site.rglob('*.html'):
    parsed = Links()
    parsed.feed(html.read_text())
    for reference in parsed.references:
        url = urlsplit(reference)
        if url.scheme or url.netloc:
            continue
        if not url.path:
            if url.fragment and unquote(url.fragment) not in parsed.ids:
                errors.append(f'Broken anchor in {html.name}: {reference}')
            continue
        if url.path.startswith('/'):
            errors.append(f'Use a relative URL for project Pages: {reference}')
            continue
        target = (html.parent / unquote(url.path)).resolve()
        if not target.is_relative_to(site.resolve()) or not target.exists():
            errors.append(f'Missing or out-of-site asset in {html.name}: {reference}')

spec = importlib.util.spec_from_file_location('demo', ROOT / 'scripts/prepare-demo.py')
demo = importlib.util.module_from_spec(spec)
spec.loader.exec_module(demo)
for demo_name, sources in demo.DEMOS.items():
    try:
        with zipfile.ZipFile(ROOT / 'docs/downloads' / (demo_name + '.zip')) as archive:
            expected = {demo_name + '/' + name for name in sources}
            if set(archive.namelist()) != expected or len(archive.namelist()) != len(expected):
                errors.append(f'{demo_name} ZIP must contain only the allowlisted demo sources.')
            for name in sources:
                if archive.read(demo_name + '/' + name) != (ROOT / 'examples' / demo_name / name).read_bytes():
                    errors.append(f'{demo_name} ZIP is stale: {name}. Run scripts/prepare-demo.py.')
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        errors.append(f'{demo_name} ZIP: {error}')

if errors:
    print('\n'.join('ERROR: ' + error for error in errors), file=sys.stderr)
    sys.exit(1)
print(f'Checked {len(files)} publication candidates, local page links, and {len(demo.DEMOS)} demo ZIPs.')
print('Private manuscript, local builds, and dependency folders are excluded.')
print('This targeted check is not a substitute for reviewing `git diff --cached` before publishing.')
