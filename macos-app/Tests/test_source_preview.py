import contextlib
import io
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, str(Path(__file__).parents[1] / 'Resources'))
import source_preview as preview
import tex_worker as worker


class SourcePreviewTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / 'project'
        self.root.mkdir()

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(text.encode('utf-8'))

    def request(self, file, text, needle, sources=None, format='latex'):
        prefix = text[:text.index(needle)]
        return dict(action='source_preview', root=str(self.root), main='main.typ' if format == 'typst' else 'main.tex',
                    format=format, engine='pdflatex', file=file, offset=len(prefix.encode('utf-16-le')) // 2,
                    sources=sources or {}, cache=str(self.base / 'cache'))

    def latex(self, text):
        return '\\documentclass{article}\n\\begin{document}\n' + text + '\n\\end{document}'

    def run_preview(self, request):
        events, rendered = [], []
        preview.preview(request, lambda **event: events.append(event), lambda request: rendered.extend(request['cards']))
        return events, rendered

    def test_exact_equation_unicode_crlf_and_no_math(self):
        text = self.latex('🧮 日本語 $a$ then $b^2$\n% $comment$\n\\verb|$raw$|\n\\$5').replace('\n', '\r\n')
        self.write('main.tex', text)
        request = self.request('main.tex', text, 'b^2')
        events, cards = self.run_preview(request)
        self.assertEqual([c['tex'] for c in cards], ['b^2'])
        start = len(text[:text.index('$b')].encode('utf-16-le')) // 2
        self.assertEqual(events[0]['previewStart'], start)
        self.assertEqual(events[0]['previewEnd'], start + len('$b^2$'))
        for needle in ['then', 'comment', 'raw', '5']:
            events, cards = self.run_preview(self.request('main.tex', text, needle))
            self.assertEqual(cards, [])
            self.assertEqual(events[0]['cards'], [])

    def test_unsaved_buffers_and_dependencies_leave_disk_unchanged(self):
        main = self.latex('\\input{parts/equations}')
        self.write('main.tex', main)
        self.write('parts/equations.tex', '$old$')
        source = 'Intro\n\\begin{align} a &= b \\\\ c &= d \\end{align}'
        request = self.request('parts/equations.tex', source, 'c &=', {'parts/equations.tex': source})
        events, cards = self.run_preview(request)
        self.assertEqual(cards[0]['kind'], 'block')
        self.assertIn('c &= d', cards[0]['tex'])
        self.assertEqual((self.root / 'parts/equations.tex').read_text(), '$old$')
        self.assertEqual((self.root / 'main.tex').read_text(), main)

    def test_typst_include_context_and_unreferenced_tex(self):
        self.write('main.typ', '#set text(size: 17pt)\n#include "part.typ"')
        source = '#let value = 3\n$ value^2 $'
        self.write('part.typ', source)
        _, cards = self.run_preview(self.request('part.typ', source, 'value^2', format='typst'))
        self.assertEqual(cards[0]['tex'], 'value^2')
        self.assertEqual([frame['file'] for frame in cards[0]['typstContext']], ['main.typ', 'part.typ'])
        self.write('main.tex', self.latex('Hello'))
        self.write('extra.tex', '$z$')
        _, cards = self.run_preview(self.request('extra.tex', '$z$', 'z'))
        self.assertEqual(cards[0]['tex'], 'z')

    def test_unrelated_deleted_draft_does_not_block_preview(self):
        main = self.latex('$x$')
        self.write('main.tex', main)
        _, cards = self.run_preview(self.request('main.tex', main, 'x$', {'deleted-draft.tex': '$unsaved$'}))
        self.assertEqual([card['tex'] for card in cards], ['x'])
        self.assertFalse((self.root / 'deleted-draft.tex').exists())
        self.assertEqual((self.root / 'main.tex').read_text(), main)

    def test_deleted_main_and_active_drafts_are_restored_only_in_snapshot(self):
        main = self.latex('\\input{deleted/equations}')
        source = '$unsaved$'
        request = self.request('deleted/equations.tex', source, 'unsaved',
                               {'main.tex': main, 'deleted/equations.tex': source})
        snapshots = []

        def render(request):
            snapshot = Path(request['root'])
            snapshots.append(snapshot)
            self.assertEqual((snapshot / 'main.tex').read_text(), main)
            self.assertEqual((snapshot / 'deleted/equations.tex').read_text(), source)
            self.assertEqual([card['tex'] for card in request['cards']], ['unsaved'])

        preview.preview(request, lambda **event: None, render)
        self.assertEqual(len(snapshots), 1)
        self.assertFalse(snapshots[0].exists())
        self.assertEqual(list(self.root.iterdir()), [])

    def test_missing_active_file_without_draft_is_rejected(self):
        self.write('main.tex', self.latex('$x$'))
        with self.assertRaisesRegex(ValueError, 'Preview file not found'):
            self.run_preview(self.request('missing.tex', '$x$', 'x'))
        self.assertFalse((self.root / 'missing.tex').exists())

    def test_rejects_outside_source_overlay(self):
        self.write('main.tex', self.latex('$a$'))
        outside = self.base / 'outside.tex'
        outside.write_text('untouched')
        (self.root / 'link.tex').symlink_to(outside)
        (self.root / 'outside-folder').symlink_to(self.base / 'missing-folder', target_is_directory=True)
        for name in ['../outside.tex', str(outside), 'link.tex', 'outside-folder/new.tex']:
            request = self.request('main.tex', self.latex('$a$'), '$a', {name: 'changed'})
            with self.assertRaises(ValueError):
                self.run_preview(request)
        self.assertEqual(outside.read_text(), 'untouched')
        self.assertFalse((self.base / 'missing-folder').exists())

    @unittest.skipUnless(os.name == 'posix', 'Process groups require POSIX')
    def test_cli_cancel_stops_compiler_descendants_and_removes_snapshot(self):
        # Run the real entry point and snapshot code with a tiny compiler stand-in.
        # A dedicated session and unconditional cleanup keep signals away from the
        # test runner even if the worker's cancellation handling regresses.
        resources = self.base / 'resources'
        resources.mkdir()
        for name in ['tex_worker.py', 'source_preview.py', 'project_outline.py']:
            shutil.copyfile(Path(worker.__file__).parent / name, resources / name)
        (resources / 'typst_worker.py').write_text('''import subprocess, sys
def extract(root, main):
    return [dict(id='preview-test', file=main, tex='x', sourceOffset=0, sourceEnd=3)], [], [main]
def render(request, emit, env):
    subprocess.run([sys.executable, '-B', request['testCompiler'], request['testState'], request['root']], check=True)
''')
        compiler = self.base / 'compiler.py'
        compiler.write_text('''import json, os, pathlib, subprocess, sys, time
child = subprocess.Popen([sys.executable, '-B', '-c', 'import time; time.sleep(30)'])
pathlib.Path(sys.argv[1]).write_text(json.dumps(dict(compiler=os.getpid(), descendant=child.pid, group=os.getpgrp(), snapshot=sys.argv[2])))
child.wait()
''')
        self.write('main.typ', '$x$')
        state_path = self.base / 'compiler-state.json'
        request = self.request('main.typ', '$x$', 'x', format='typst')
        request.update(testCompiler=str(compiler), testState=str(state_path))
        request_path = self.base / 'request.json'
        request_path.write_text(json.dumps(request))
        temporary = self.base / 'worker-temporary'
        temporary.mkdir()
        process = subprocess.Popen([sys.executable, '-B', str(resources / 'tex_worker.py'), str(request_path)],
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True,
                                   env=dict(os.environ, TMPDIR=str(temporary), TMP=str(temporary), TEMP=str(temporary)))
        state = None
        try:
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if state_path.exists():
                    try:
                        state = json.loads(state_path.read_text())
                        break
                    except json.JSONDecodeError:
                        pass
                if process.poll() is not None:
                    break
                time.sleep(0.02)
            self.assertIsNotNone(state, 'Preview compiler did not start')
            self.assertEqual(state['group'], process.pid)
            self.assertTrue(Path(state['snapshot']).exists())
            process.terminate()
            stdout, stderr = process.communicate(timeout=5)
            self.assertEqual(process.returncode, 128 + signal.SIGTERM, (stdout, stderr))
            self.assertFalse(Path(state['snapshot']).parent.exists())
            # The grandchild inherits stdout/stderr, so communicate() also times
            # out if it survives; check its process state explicitly as well.
            for pid in [state['compiler'], state['descendant']]:
                result = subprocess.run(['ps', '-o', 'stat=', '-p', str(pid)], capture_output=True, text=True)
                self.assertTrue(result.returncode != 0 or result.stdout.strip().startswith('Z'),
                                'Compiler descendant still running: ' + str(pid))
            self.assertEqual((self.root / 'main.typ').read_text(), '$x$')
        finally:
            # The process was started as a session leader, so this group belongs
            # only to this test, including in a failed cancellation regression.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.communicate(timeout=5)
            if state is not None:
                shutil.rmtree(Path(state['snapshot']).parent, ignore_errors=True)

    @unittest.skipUnless(os.environ.get('SLIDEMATH_COMPILER_TESTS') == '1', 'Compiler integration disabled')
    def test_real_compilers_use_unsaved_project_definitions(self):
        env = worker.tool_env()
        for binary in ['typst', 'latexmk', 'pdf2svg', 'pdftocairo', 'pdfcrop']:
            if not shutil.which(binary, path=env['PATH']):
                self.skipTest(binary + ' is required')
        latex = '\\documentclass{article}\n\\input{commands}\n\\begin{document}\n$\\previewmacro$\n\\end{document}'
        self.write('main.tex', latex)
        self.write('main.typ', '#include "part.typ"')
        typst = '#import "symbols.typ": previewvalue\n$ previewvalue^2 $'
        self.write('part.typ', typst)
        # Deleted dependency drafts must be restored only in the snapshot used
        # by the real compilers; neither file exists in the original project.
        requests = [self.request('main.tex', latex, 'previewmacro', {'commands.tex': '\\newcommand{\\previewmacro}{x^2}'}),
                    self.request('part.typ', typst, 'previewvalue^2', {'symbols.typ': '#let previewvalue = 7'}, format='typst')]
        original = {path: path.read_bytes() for path in self.root.rglob('*') if path.is_file()}
        for request in requests:
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                worker.main(request)
            events = [json.loads(line) for line in output.getvalue().splitlines()]
            cards = [event for event in events if event['event'] == 'card']
            self.assertEqual(len(cards), 1, events)
            self.assertEqual(cards[0]['status'], 'ready', events)
            for key in ['svgPath', 'pngPath', 'pdfPath']:
                self.assertTrue(Path(cards[0][key]).is_file())
        self.assertEqual(original, {path: path.read_bytes() for path in self.root.rglob('*') if path.is_file()})
