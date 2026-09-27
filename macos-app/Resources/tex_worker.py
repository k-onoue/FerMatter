#!/usr/bin/env python3
"""Local TeX extraction/render worker. JSON request in a file; JSON lines on stdout."""
import collections
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
import xml.etree.ElementTree as ET
from project_outline import Outline, tex_title

VERSION = '5'
MATH_ENVS = {'equation', 'equation*', 'align', 'align*', 'gather', 'gather*', 'multline', 'multline*', 'displaymath', 'math'}
SKIP_ENVS = {'verbatim', 'verbatim*', 'Verbatim', 'lstlisting', 'minted', 'comment'}

def emit(**data):
    print(json.dumps(data, ensure_ascii=False), flush=True)

def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()[:24]

def without_comments(text):
    chars = list(text)
    for match in re.finditer('%', text):
        start = match.start()
        slashes = len(text[:start]) - len(text[:start].rstrip('\\'))
        if slashes % 2 == 0:
            end = text.find('\n', start)
            end = len(text) if end < 0 else end
            chars[start:end] = ' ' * (end - start)
    return ''.join(chars)

def balanced_end(text, start):
    depth = 0
    for i in range(start, len(text)):
        if i and text[i - 1] == '\\':
            continue
        if text[i] == '{':
            depth += 1
        elif text[i] == '}':
            depth -= 1
            if depth == 0:
                return i + 1
    return len(text)

def resolve_tex(root, current, name):
    if '\\' in name or '#' in name:
        raise ValueError('Dynamic input is not supported: ' + name)
    for base in (current.parent, root):
        path = (base / name).with_suffix('.tex') if not Path(name).suffix else base / name
        path = path.resolve()
        if path.is_file() and (path == root or root in path.parents):
            return path
    raise ValueError('Referenced file not found: ' + name)

def extract(root, main, outline=None):
    root = Path(root).resolve()
    cards, warnings, files = [], [], set()
    occurrences = collections.Counter()
    document = Outline(outline)

    def walk(path, stack=(), start_at=0):
        if path in stack or len(stack) > 32:
            warnings.append('Skipped cyclic input: ' + str(path.relative_to(root)))
            return
        raw = path.read_text(encoding='utf-8-sig')
        if len(raw) > 10_000_000:
            raise ValueError('TeX file is too large.')
        text = without_comments(raw)
        lines = raw.splitlines()
        file = str(path.relative_to(root))
        files.add(file)
        i = start_at
        while i < len(text):
            if text.startswith('\\end{document}', i):
                break
            macro = re.match(r'\\(?:newcommand|renewcommand|providecommand|DeclareMathOperator|DeclareRobustCommand)\*?\s*', text[i:])
            if macro:
                cursor = i + macro.end()
                if cursor < len(text) and text[cursor] == '{':
                    cursor = balanced_end(text, cursor)
                else:
                    command = re.match(r'\\[A-Za-z@]+', text[cursor:])
                    if command:
                        cursor += command.end()
                while cursor < len(text) and (text[cursor].isspace() or text[cursor] == '['):
                    if text[cursor] == '[':
                        end = text.find(']', cursor + 1)
                        cursor = end + 1 if end >= 0 else len(text)
                    else:
                        cursor += 1
                i = balanced_end(text, cursor) if cursor < len(text) and text[cursor] == '{' else cursor
                continue
            definition = re.match(r'\\(?:def|gdef|edef|xdef)(?![A-Za-z@])', text[i:])
            if definition:
                cursor = text.find('{', i + definition.end())
                i = balanced_end(text, cursor) if cursor >= 0 else len(text)
                continue
            heading = re.match(r'\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)(?![A-Za-z@])\*?\s*', text[i:])
            if heading:
                cursor = i + heading.end()
                if cursor < len(text) and text[cursor] == '[':
                    depth = 1; cursor += 1
                    while cursor < len(text) and depth:
                        if text[cursor] == '\\': cursor += 2; continue
                        if text[cursor] == '{': cursor = balanced_end(text, cursor); continue
                        if text[cursor] == '[': depth += 1
                        if text[cursor] == ']': depth -= 1
                        cursor += 1
                    while cursor < len(text) and text[cursor].isspace(): cursor += 1
                if cursor < len(text) and text[cursor] == '{':
                    end = balanced_end(text, cursor)
                    level = ('part', 'chapter', 'section', 'subsection', 'subsubsection', 'paragraph', 'subparagraph').index(heading[1])
                    document.add(tex_title(text[cursor + 1:end - 1]), level, file, raw.count('\n', 0, i) + 1)
                    i = end
                    continue
            include = re.match(r'\\(?:input|include)(?![A-Za-z@])\s*(?:\{([^}]+)\}|([^\s{}]+))', text[i:])
            if include:
                try:
                    dependency = resolve_tex(root, path, (include[1] or include[2]).strip())
                    walk(dependency, stack + (path,))
                except ValueError as error:
                    warnings.append(str(error))
                i += include.end()
                continue
            verb = re.match(r'\\verb\*?([^A-Za-z\s])', text[i:])
            if verb:
                end = text.find(verb[1], i + verb.end())
                i = len(text) if end < 0 else end + 1
                continue
            env = re.match(r'\\begin\s*\{([^}]+)\}', text[i:])
            if env and env[1] in SKIP_ENVS:
                close = re.search(r'\\end\s*\{' + re.escape(env[1]) + r'\}', text[i + env.end():])
                i = i + env.end() + close.end() if close else len(text)
                continue
            kind, end, body = None, None, None
            if env and env[1] in MATH_ENVS | {'tikzpicture'}:
                name = env[1]
                tokens = re.compile(r'\\(begin|end)\s*\{' + re.escape(name) + r'\}')
                depth = 1
                for token in tokens.finditer(text, i + env.end()):
                    depth += 1 if token[1] == 'begin' else -1
                    if depth == 0:
                        end = token.end()
                        break
                kind = 'figure' if name == 'tikzpicture' else 'inline' if name == 'math' else 'block'
                if end:
                    body = raw[i:end]
            else:
                opening = next((d for d in ('$$', '\\[', '\\(', '$') if text.startswith(d, i)), None)
                if opening:
                    closing = {'$$': '$$', '$': '$', '\\[': '\\]', '\\(': '\\)'}[opening]
                    cursor = i + len(opening)
                    while cursor < len(text):
                        if text.startswith(closing, cursor):
                            end = cursor + len(closing)
                            body = raw[i + len(opening):cursor]
                            break
                        cursor += 2 if text[cursor] == '\\' else 1
                    kind = 'inline' if opening in ('$', '\\(') else 'block'
            if kind and end:
                line = raw.count('\n', 0, i) + 1
                end_line = raw.count('\n', 0, end) + 1
                tex = body.strip()
                key = file + '\0' + re.sub(r'\s+', ' ', tex)
                occurrences[key] += 1
                label = re.search(r'\\label\{([^}]+)\}', tex)
                name = label[1].split(':', 1)[-1].replace('_', ' ').replace('-', ' ').capitalize() if label else ('TikZ · ' + str(line) if kind == 'figure' else 'Inline · ' + tex[:45] if kind == 'inline' else 'Equation · ' + str(line))
                cards.append(dict(id='source-' + digest(key + str(occurrences[key])), name=name, tex=tex,
                                  kind=kind, origin='source', file=file, line=line, endLine=end_line,
                                  column=i - raw.rfind('\n', 0, i), endColumn=end - raw.rfind('\n', 0, end),
                                  sourceOrder=float(len(cards)), contextStart=max(1, line - 5),
                                  context=lines[max(0, line - 6):end_line + 5], outlinePath=document.path, status='pending'))
                i = end
                continue
            if kind and not end:
                warnings.append(file + ':' + str(raw.count('\n', 0, i) + 1) + ' Unclosed equation.')
            # Skip escaped dollars and other one-character commands as one token.
            i += 2 if text[i] == '\\' and i + 1 < len(text) and not text[i + 1].isalpha() else 1

    main_path = (root / main).resolve()
    if root not in main_path.parents:
        raise ValueError('Main file must be inside the project folder.')
    content = without_comments(main_path.read_text(encoding='utf-8-sig'))
    begin = re.search(r'\\begin\s*\{document\}', content)
    if not begin:
        raise ValueError('Main file has no begin{document}.')
    walk(main_path, start_at=begin.end())
    return cards, warnings, sorted(files)

def project_fingerprint(root):
    hashes = []
    for path in sorted(root.rglob('*')):
        if any(part in ('.git', '.build', 'node_modules', '__MACOSX') for part in path.parts):
            continue
        if path.is_file() and (path.suffix in ('.tex', '.sty', '.cls', '.cfg', '.def') or path.name in ('latexmkrc', '.latexmkrc')):
            hashes.append(str(path.relative_to(root)) + hashlib.sha256(path.read_bytes()).hexdigest())
    return digest('\n'.join(hashes))

def tool_env():
    env = os.environ.copy()
    env['PATH'] = '/Library/TeX/texbin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:' + env.get('PATH', '')
    return env

def run(command, cwd, env, timeout=90):
    result = subprocess.run(command, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(result.stdout[-14000:])
    return result.stdout

def math_body(card):
    if card['kind'] == 'figure':
        return card['tex'].strip()
    tex = re.sub(r'\n[ \t]*\n+', '\n', without_comments(card['tex'])).strip()
    tex = re.sub(r'\\(?:label|tag\*?)\{[^}]*\}', '', tex)
    tex = re.sub(r'\\(begin|end)\{(equation|align|gather|multline)\*?\}', lambda m: '\\' + m[1] + '{' + m[2] + '*}', tex)
    if re.match(r'\\begin\{(?:equation|align|gather|multline|displaymath|math)\*?\}', tex):
        return re.sub(r'\n[ \t]*\n+', '\n', tex)
    for opening, closing in [('$$', '$$'), ('\\[', '\\]'), ('\\(', '\\)'), ('$', '$')]:
        if tex.startswith(opening) and tex.endswith(closing):
            tex = tex[len(opening):-len(closing)].strip()
            break
    return '$' + tex + '$' if card['kind'] == 'inline' else '\\[' + tex + '\\]'

def render(request):
    root = Path(request['root']).resolve()
    cache = Path(request['cache']).resolve()
    cache.mkdir(parents=True, exist_ok=True)
    env = tool_env()
    for binary in ('latexmk', 'pdf2svg', 'pdftocairo', 'pdfinfo', 'pdfcrop'):
        if not shutil.which(binary, path=env['PATH']):
            raise ValueError(binary + ' is required.')
    rc = next((name for name in ('latexmkrc', '.latexmkrc') if (root / name).exists()), None)
    engine = request.get('engine', 'project')
    if not rc and engine == 'project':
        raise ValueError('No latexmkrc found. Select a compiler in Project Settings.')
    fingerprint = project_fingerprint(root)
    pending = []
    for card in request['cards']:
        key = digest(VERSION + fingerprint + request['main'] + engine + card['kind'] + card['tex'])
        card = dict(card, cacheKey=key)
        svg, png = cache / (key + '.svg'), cache / (key + '.png')
        single_pdf = cache / (key + '.pdf')
        font_size = None
        if svg.exists():
            try:
                font_size = float(ET.parse(svg).getroot().get('data-tex-font-size', '0')) or None
            except (ET.ParseError, ValueError):
                pass
        if svg.exists() and png.exists() and single_pdf.exists() and (not request.get('requireFontMetrics') or font_size):
            emit(event='card', id=card['id'], status='ready', svgPath=str(svg), pngPath=str(png), pdfPath=str(single_pdf), fontSize=font_size)
        else:
            pending.append(card)
    if not pending:
        return
    raw = (root / request['main']).read_text(encoding='utf-8-sig')
    match = re.search(r'\\begin\s*\{document\}', without_comments(raw))
    if not match:
        raise ValueError('Main file has no begin{document}.')
    preamble = raw[:match.start()]
    # All generated sources and outputs stay in a disposable copy, never in the manuscript.
    with tempfile.TemporaryDirectory(prefix='slide-math-') as temporary:
        stage = Path(temporary) / 'project'
        shutil.copytree(root, stage, ignore=shutil.ignore_patterns('.git', '.build', 'node_modules', '*.aux', '*.log', '*.synctex.gz', '*.fls', '*.fdb_latexmk'), symlinks=False)

        def batch(group):
            job = 'sm_' + digest(''.join(c['id'] for c in group))
            output = Path(temporary) / job
            output.mkdir()
            # preview gives one tightly cropped page per equation; no equation numbers.
            document = ('\\PassOptionsToPackage{active,tightpage}{preview}\n' + preamble +
                        ('\n\\usepackage{tikz}\n' if any(c['kind'] == 'figure' for c in group) else '') +
                        '\n\\usepackage{preview}\n\\pagestyle{empty}\n\\begin{document}\n' +
                        '\\ifcsname ClearShipoutPictureBG\\endcsname\\ClearShipoutPictureBG\\fi\n' +
                        '\\ifcsname ClearShipoutPictureFG\\endcsname\\ClearShipoutPictureFG\\fi\n' +
                        '\\ifcsname nolinenumbers\\endcsname\\nolinenumbers\\fi\n' +
                        '\\pagestyle{empty}\n\\ifcsname tikzexternaldisable\\endcsname\\tikzexternaldisable\\fi\n' +
                        '\\makeatletter\\typeout{SLIDEMATHFONT:\\f@size}\\makeatother\n' +
                        '\n'.join('\\begin{preview}\n' + math_body(c) + '\n\\end{preview}' for c in group) + '\n\\end{document}\n')
            (stage / (job + '.tex')).write_text(document)
            command = ['latexmk', '-norc']
            if rc and engine == 'project':
                command += ['-r', rc]
            elif engine in ('pdflatex', 'xelatex', 'lualatex'):
                # Still load the project's path/package configuration before an explicit engine override.
                if rc:
                    command += ['-r', rc]
                command += [{'pdflatex': '-pdf', 'xelatex': '-xelatex', 'lualatex': '-lualatex'}[engine]]
            else:
                raise ValueError('Select a compiler in Project Settings.')
            command += ['-interaction=nonstopmode', '-halt-on-error', '-file-line-error', '-latexoption=-no-shell-escape', '-outdir=' + str(output), job + '.tex']
            try:
                run(command, stage, env)
                log_text = (output / (job + '.log')).read_text(errors='replace')
                font_match = re.search(r'SLIDEMATHFONT:([0-9.]+)', log_text)
                font_size = float(font_match[1]) if font_match else None
                pdf = output / (job + '.pdf')
                if not pdf.exists() and (output / (job + '.dvi')).exists():
                    ps = output / (job + '.ps')
                    run(['dvips', '-o', str(ps), str(output / (job + '.dvi'))], stage, env)
                    run(['ps2pdf', '-dEPSCrop', str(ps), str(pdf)], stage, env)
                if not pdf.exists():
                    raise RuntimeError('Compiler produced no PDF or DVI.')
                cropped = output / (job + '_crop.pdf')
                run(['pdfcrop', '--margins', '2', str(pdf), str(cropped)], stage, env)
                pdf = cropped
                info = run(['pdfinfo', str(pdf)], stage, env)
                pages = re.search(r'^Pages:\s+(\d+)', info, re.M)
                if not pages or int(pages[1]) != len(group):
                    raise RuntimeError('Rendered page count does not match the equation count.')
            except (RuntimeError, subprocess.TimeoutExpired) as error:
                if len(group) > 1:
                    half = len(group) // 2
                    batch(group[:half]); batch(group[half:])
                else:
                    log = cache / (group[0]['cacheKey'] + '.log')
                    log.write_text(str(error))
                    emit(event='card', id=group[0]['id'], status='failed', error=str(error)[-5000:], logPath=str(log))
                return
            for index, card in enumerate(group, 1):
                svg, png = cache / (card['cacheKey'] + '.svg'), cache / (card['cacheKey'] + '.png')
                single_pdf = cache / (card['cacheKey'] + '.pdf')
                try:
                    run(['pdftocairo', '-pdf', '-origpagesizes', '-f', str(index), '-l', str(index), str(pdf), str(single_pdf)], stage, env, timeout=30)
                    run(['pdf2svg', str(pdf), str(svg), str(index)], stage, env, timeout=30)
                    if font_size:
                        svg.write_text(svg.read_text().replace('<svg ', '<svg data-tex-font-size="' + str(font_size) + '" ', 1))
                    run(['pdftocairo', '-png', '-transp', '-singlefile', '-scale-to', '900', '-f', str(index), '-l', str(index), str(pdf), str(png.with_suffix(''))], stage, env, timeout=30)
                    emit(event='card', id=card['id'], status='ready', svgPath=str(svg), pngPath=str(png), pdfPath=str(single_pdf), fontSize=font_size)
                except (RuntimeError, subprocess.TimeoutExpired) as error:
                    emit(event='card', id=card['id'], status='failed', error=str(error)[-5000:])
        for i in range(0, len(pending), 32):
            batch(pending[i:i + 32])

def import_zip(request):
    archive = Path(request['path'])
    destination = Path(request['destination']) / digest(str(archive.resolve()) + str(archive.stat().st_mtime_ns))
    if not destination.exists():
        with zipfile.ZipFile(archive) as z:
            members = z.infolist()
            if len(members) > 10000 or sum(m.file_size for m in members) > 512 * 1024 * 1024:
                raise ValueError('Archive exceeds 512 MB or 10,000 files.')
            for member in members:
                target = (destination / member.filename).resolve()
                if destination.resolve() not in target.parents or ((member.external_attr >> 16) & 0o170000) == 0o120000:
                    raise ValueError('Archive contains an invalid path or symbolic link.')
            destination.mkdir(parents=True)
            z.extractall(destination)
    children = [p for p in destination.iterdir() if p.name != '__MACOSX' and not p.name.startswith('.')]
    if len(children) == 1 and children[0].is_dir():
        destination = children[0]
    emit(event='imported', root=str(destination))

def main(request):
    if request['action'] == 'scan':
        import typst_worker
        root = Path(request['root']).resolve()
        tex_candidates, typst_candidates = [], []
        for path in root.rglob('*'):
            if not path.is_file() or any(part in typst_worker.IGNORED for part in path.relative_to(root).parts):
                continue
            if not path.resolve().is_relative_to(root):
                continue
            if path.suffix == '.typ':
                typst_candidates.append(path.relative_to(root).as_posix())
            if path.suffix != '.tex':
                continue
            if '\\begin{document}' in without_comments(path.read_text(encoding='utf-8-sig', errors='replace')):
                tex_candidates.append(path.relative_to(root).as_posix())
        selected = request.get('main', '')
        project_format = request.get('format') or ('typst' if selected.endswith('.typ') else 'latex' if selected.endswith('.tex') else None)
        if not project_format:
            if tex_candidates and typst_candidates:
                raise ValueError('This folder contains both LaTeX and Typst. Open a .tex or .typ main file to choose the project format.')
            project_format = 'typst' if typst_candidates else 'latex'
        if project_format not in ('latex', 'typst'):
            raise ValueError('Unknown project format.')
        candidates = typst_candidates if project_format == 'typst' else tex_candidates
        preferred = 'main.typ' if project_format == 'typst' else 'main.tex'
        candidates.sort(key=lambda name: (name != preferred, len(Path(name).parts), name))
        selected = request.get('main') or (candidates[0] if candidates else '')
        if not selected or selected not in candidates:
            raise ValueError('No matching main file found. Open a LaTeX or Typst project folder or main file.')
        outline = []
        cards, warnings, files = (typst_worker.extract if project_format == 'typst' else extract)(root, selected, outline=outline)
        emit(event='scanned', cards=cards, candidates=candidates, main=selected, warnings=warnings,
             files=files, outline=outline, format=project_format, hasRC=any((root / name).exists() for name in ('latexmkrc', '.latexmkrc')))
    elif request['action'] == 'render':
        if request.get('format') == 'typst' or request.get('main', '').endswith('.typ'):
            import typst_worker
            typst_worker.render(request, emit, tool_env())
        else:
            render(request)
    elif request['action'] == 'pdf_build':
        import document_worker
        document_worker.build(request, emit, tool_env())
    elif request['action'] in ('pdf_forward', 'pdf_reverse'):
        import document_worker
        document_worker.sync(request, emit, tool_env())
    elif request['action'] == 'import':
        import_zip(request)
    else:
        raise ValueError('Unknown action')
    emit(event='done')

if __name__ == '__main__':
    try:
        main(json.loads(Path(sys.argv[1]).read_text()))
    except Exception as error:
        emit(event='error', message=str(error))
        sys.exit(1)
