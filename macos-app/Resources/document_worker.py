"""Whole-document PDF builds and source navigation; originals are never modified."""
import collections
import hashlib
import json
import math
from pathlib import Path
import re
import shutil
import subprocess
import uuid

import typst_worker

VERSION = 'pdf-3'
GENERATED = {'.aux', '.log', '.out', '.toc', '.fls', '.fdb_latexmk', '.blg', '.nav', '.snm', '.vrb', '.xdv', '.dvi'}


def run(command, cwd, env, timeout=180):
    # SyncTeX supports launching arbitrary viewers via environment variables.
    # This worker only reads its output; never let those hooks launch anything.
    env = dict(env)
    env.pop('SYNCTEX_EDITOR', None)
    env.pop('SYNCTEX_VIEWER', None)
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise ValueError((result.stdout + '\n' + result.stderr)[-16000:])
    return result.stdout


def fingerprint(root, main):
    digest = hashlib.sha256()
    output_name = Path(main).with_suffix('.pdf').name
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root)
        if any(part in typst_worker.IGNORED for part in relative.parts) or not path.is_file():
            continue
        if path.suffix in GENERATED or path.name == output_name:
            continue
        digest.update(relative.as_posix().encode())
        with path.open('rb') as stream:
            for block in iter(lambda: stream.read(65536), b''):
                digest.update(block)
    return digest.hexdigest()


def source_copy(root, target):
    shutil.copytree(root, target, symlinks=False, ignore=shutil.ignore_patterns(
        *typst_worker.IGNORED, '*.aux', '*.log', '*.out', '*.toc', '*.fls', '*.fdb_latexmk', '*.synctex.gz'))


def build(request, emit, env):
    root = Path(request['root']).resolve()
    main = request['main']
    typst_worker.safe_path(root, main)
    project_format = 'typst' if main.endswith('.typ') else 'latex'
    compiler = 'typst' if project_format == 'typst' else 'latexmk'
    executable = shutil.which(compiler, path=env['PATH'])
    if not executable:
        raise ValueError(f'{compiler} is required to compile the document PDF.')
    version = run([executable, '--version' if project_format == 'typst' else '-v'], root, env)
    key = typst_worker.digest(VERSION + main + request.get('engine', 'project') + fingerprint(root, main) + version)
    cache = Path(request['cache']).resolve()
    cache.mkdir(parents=True, exist_ok=True)
    manifest = cache / (key + '.json')
    if not request.get('force') and manifest.is_file():
        document = json.loads(manifest.read_text())
        if Path(document['pdfPath']).is_file() and Path(document['sourceRoot']).is_dir():
            emit(event='document', document=document)
            return
    # A rebuild gets a fresh directory, so a failed build cannot damage the PDF
    # currently displayed by PDFKit. SyncTeX's source paths remain valid here.
    destination = cache / (key + '-' + uuid.uuid4().hex[:8])
    stage, output = destination / 'Source', destination / 'Output'
    output.mkdir(parents=True)
    try:
        source_copy(root, stage)
        if project_format == 'typst':
            pdf, anchors, warnings = build_typst(stage, main, output, executable, env)
            synchronized = bool(anchors)
        else:
            pdf, synchronized, warnings = build_tex(stage, main, output, executable, request.get('engine', 'project'), env)
            anchors = []
        document = dict(pdfPath=str(pdf), sourceRoot=str(stage), format=project_format,
                        synchronized=synchronized, anchors=anchors, warnings=warnings)
        manifest.write_text(json.dumps(document))
        emit(event='document', document=document)
    except Exception:
        # Only our new, incomplete compilation directory is disposable.
        shutil.rmtree(destination, ignore_errors=True)
        raise


def build_tex(stage, main, output, executable, engine, env):
    rc = next((name for name in ('latexmkrc', '.latexmkrc') if (stage / name).is_file()), None)
    command = [executable, '-norc']
    if rc:
        command += ['-r', rc]
    if engine in ('pdflatex', 'xelatex', 'lualatex'):
        command += [{'pdflatex': '-pdf', 'xelatex': '-xelatex', 'lualatex': '-lualatex'}[engine]]
    elif engine != 'project' or not rc:
        raise ValueError('Select a LaTeX compiler in Project Settings.')
    command += ['-synctex=1', '-interaction=nonstopmode', '-halt-on-error', '-file-line-error',
                '-latexoption=-no-shell-escape', '-outdir=' + str(output), main]
    run(command, stage, env)
    pdf = output / (Path(main).stem + '.pdf')
    if not pdf.exists() and (dvi := pdf.with_suffix('.dvi')).exists():
        ps = pdf.with_suffix('.ps')
        run(['dvips', '-o', str(ps), str(dvi)], stage, env)
        run(['ps2pdf', str(ps), str(pdf)], stage, env)
    if not pdf.is_file():
        raise ValueError('The compiler did not produce a PDF.')
    synchronized = bool(shutil.which('synctex', path=env['PATH']) and
                        (pdf.with_suffix('.synctex.gz').exists() or pdf.with_suffix('.synctex').exists()))
    warnings = [] if synchronized else ['Source navigation needs a SyncTeX-enabled build and the synctex command.']
    return pdf, synchronized, warnings


def build_typst(stage, main, output, executable, env):
    headings = []
    cards, warnings, _ = typst_worker.extract(stage, main, outline=headings)
    targets = collections.defaultdict(list)
    edits = collections.defaultdict(dict)
    namespace = 'slide-math-nav-' + uuid.uuid4().hex
    label = 'slide-math-map-' + uuid.uuid4().hex
    for card in cards:
        file = card['file']
        start = card.get('sourceOffset', card['typstContext'][-1]['offset'])
        text = (stage / file).read_text(encoding='utf-8-sig')
        offset = start + 1
        while offset < len(text) and text[offset].isspace():
            offset += 1
        key = file + ':' + str(start)
        targets[key].append(card['id'])
        marker = 'metadata((' + json.dumps(namespace) + ', ' + json.dumps(key) + '))'
        if card['kind'] == 'figure':
            edits[file][start] = 'context { let slide-math-drawing = '
            edits[file][card['sourceEnd']] = ('; let slide-math-size = measure(slide-math-drawing); metadata((' +
                json.dumps(namespace) + ', ' + json.dumps(key) +
                ', slide-math-size.width.pt(), slide-math-size.height.pt())); slide-math-drawing }')
        else:
            edits[file][offset] = '#' + marker + ' '
    for heading in headings:
        file = heading['file']
        text = (stage / file).read_text(encoding='utf-8-sig')
        offset = sum(len(line) for line in text.splitlines(keepends=True)[:heading['line'] - 1])
        # A marker just before a heading preserves its original source title.
        key = file + ':heading:' + str(offset)
        targets[key].append(heading['id'])
        edits[file][offset] = '#metadata((' + json.dumps(namespace) + ', ' + json.dumps(key) + '))\n'
    for file, insertions in edits.items():
        path = stage / file
        text = path.read_text(encoding='utf-8-sig')
        for offset, insertion in sorted(insertions.items(), reverse=True):
            text = text[:offset] + insertion + text[offset:]
        path.write_text(text)
    query = '''\n#context [#metadata(query(metadata).filter(it => type(it.value) == array and (it.value.len() == 2 or it.value.len() == 4) and it.value.at(0) == NAMESPACE).map(it => {
 let p = it.location().position()
 (key: it.value.at(1), page: p.page, x: p.x.pt(), y: p.y.pt(), width: if it.value.len() == 4 { it.value.at(2) } else { 0 }, height: if it.value.len() == 4 { it.value.at(3) } else { 0 })
})) <LABEL>]\n'''.replace('NAMESPACE', json.dumps(namespace)).replace('LABEL', label)
    entry = stage / main
    entry.write_text(entry.read_text() + query)
    pdf = output / 'document.pdf'
    common = ['--root', str(stage), '--font-path', str(stage)]
    run([executable, 'compile', *common, str(entry), str(pdf)], stage, env)
    records = json.loads(run([executable, 'query', *common, str(entry), '<' + label + '>', '--field', 'value', '--one'], stage, env))
    indices = collections.Counter()
    anchors = []
    for record in records:
        key = record['key']; index = indices[key]; indices[key] += 1
        if index < len(targets[key]):
            anchors.append(dict(targetID=targets[key][index], page=record['page'], x=record['x'], y=record['y'], width=record['width'], height=record['height']))
    if len(anchors) != len(cards) + len(headings):
        warnings.append('Some generated or repeated content has no unambiguous source position.')
    return pdf, anchors, warnings


def synctex_records(text):
    records, current = [], {}
    for line in text.splitlines():
        match = re.match(r'^(Output|Page|Input|Line|Column|x|y|h|v|W|H):(.*)$', line)
        if not match:
            continue
        key, value = match.groups()
        if key == 'Output' and current:
            records.append(current); current = {}
        current[key] = value.strip()
    if current:
        records.append(current)
    return records


def sync(request, emit, env):
    document = request['document']
    source = Path(document['sourceRoot']).resolve()
    pdf = Path(document['pdfPath']).resolve()
    if not pdf.is_file() or not source.is_dir() or not document.get('synchronized'):
        raise ValueError('Compile the PDF before using source navigation.')
    if request['action'] == 'pdf_forward':
        if document['format'] == 'typst':
            anchor = next((a for a in document['anchors'] if a['targetID'] == request['targetID']), None)
            if anchor:
                emit(event='pdf_location', anchor=anchor)
            else:
                emit(event='pdf_unmapped')
            return
        file = typst_worker.safe_path(source, request['file'])
        location = f"{int(request['line'])}:{int(request.get('column') or 0)}:{file}"
        records = synctex_records(run(['synctex', 'view', '-i', location, '-o', str(pdf)], source, env, 15))
        for record in records:
            if all(key in record for key in ('Page', 'x', 'y')):
                emit(event='pdf_location', anchor=dict(targetID=request['targetID'], page=int(record['Page']), x=float(record['x']), y=float(record['y'])))
                return
        emit(event='pdf_unmapped')
    else:
        page, x, y = int(request['page']), float(request['x']), float(request['y'])
        if page < 1 or not all(math.isfinite(v) for v in (x, y)):
            raise ValueError('Invalid PDF position.')
        if document['format'] == 'typst':
            figures = [a for a in document['anchors'] if a['page'] == page and a.get('width', 0) > 0 and
                       a['x'] - 3 <= x <= a['x'] + a['width'] + 3 and a['y'] - 3 <= y <= a['y'] + a['height'] + 3]
            anchors = figures or [a for a in document['anchors'] if a['page'] == page and abs(a['y'] - y) <= 35]
            # Prioritize the line, then horizontal distance within it.
            anchor = min(anchors, key=lambda a: abs(a['y'] - y) * 4 + abs(a['x'] - x), default=None)
            if anchor:
                emit(event='pdf_target', id=anchor['targetID'])
            else:
                emit(event='pdf_unmapped')
            return
        records = synctex_records(run(['synctex', 'edit', '-o', f'{page}:{x}:{y}:{pdf}'], source, env, 15))
        for record in records:
            if 'Input' in record and 'Line' in record:
                path = (source / record['Input']).resolve()
                if path.is_relative_to(source):
                    emit(event='pdf_source', file=path.relative_to(source).as_posix(), line=int(record['Line']), column=int(record.get('Column', '-1')))
                    return
        emit(event='pdf_unmapped')
