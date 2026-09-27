"""Resolve the equation at an editor caret using an isolated source snapshot."""
from bisect import bisect_right
from pathlib import Path
import shutil
import tempfile

import tex_worker
import typst_worker


def project_file(root, name, must_exist=True):
    root = root.resolve()
    path = Path(name)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError('Preview files must be inside the project.')
    target = (root / path).resolve()
    if target == root or not target.is_relative_to(root):
        raise ValueError('Preview files must be inside the project.')
    if (must_exist or target.exists()) and not target.is_file():
        raise ValueError('Preview file not found in the project: ' + name)
    return target


def utf16_boundaries(text):
    """Map Python's newline-normalized offsets to NSTextView UTF-16 positions."""
    positions = [0]
    i = units = 0
    while i < len(text):
        char = text[i]
        units += 2 if ord(char) > 0xffff else 1
        i += 1
        if char == '\r' and i < len(text) and text[i] == '\n':
            i += 1
            units += 1
        positions.append(units)
    return positions


def preview(request, emit, render):
    root = Path(request['root']).resolve()
    file = request['file']
    # A recovered draft can outlive its file or containing folder on disk.
    # Validate containment here; require existence only after applying drafts.
    project_file(root, file, must_exist=False)
    project_file(root, request['main'], must_exist=False)
    sources = request.get('sources', {})
    for name, text in sources.items():
        project_file(root, name, must_exist=False)
        if not isinstance(text, str):
            raise ValueError('Invalid source buffer.')

    # No editor operation, including manual-save mode, writes to the manuscript.
    # Relative imports, local packages and compiler settings travel together.
    with tempfile.TemporaryDirectory(prefix='mathlidelux-source-preview-') as temporary:
        snapshot = Path(temporary) / 'project'
        shutil.copytree(root, snapshot, symlinks=False, ignore=shutil.ignore_patterns(
            '.git', '.build', 'build', 'node_modules', '__pycache__', '*.aux',
            '*.log', '*.synctex.gz', '*.fls', '*.fdb_latexmk'))
        for name, text in sources.items():
            target = project_file(snapshot, name, must_exist=False)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding='utf-8')
        project_file(snapshot, request['main'])
        with project_file(snapshot, file).open(encoding='utf-8-sig', newline='') as stream:
            text = stream.read()
        boundaries = utf16_boundaries(text)
        offset = max(0, min(int(request['offset']), boundaries[-1]))
        position = bisect_right(boundaries, offset) - 1
        extract = typst_worker.extract if request['format'] == 'typst' else tex_worker.extract
        cards, _, files = extract(snapshot, request['main'])
        # An open companion file can contain math before it is included in main.
        if file not in files:
            if request['format'] == 'typst':
                cards, _, _ = extract(snapshot, file)
            else:
                cards, _, _ = extract(snapshot, request['main'], focus_file=file)
        card = next((card for card in cards if card['file'] == file
                     and card['sourceOffset'] <= position < card['sourceEnd']), None)
        if card is None:
            emit(event='source_preview', cards=[])
            return
        emit(event='source_preview', cards=[card],
             previewStart=boundaries[card['sourceOffset']], previewEnd=boundaries[card['sourceEnd']])
        render(dict(request, action='render', root=str(snapshot), cards=[card]))
