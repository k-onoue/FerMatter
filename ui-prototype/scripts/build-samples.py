"""Generate the browser fixture from the public, synthetic demo paper."""
import importlib.util
import json
from pathlib import Path

prototype = Path(__file__).resolve().parents[1]
repository = prototype.parent
spec = importlib.util.spec_from_file_location('worker', repository / 'macos-app/Resources/tex_worker.py')
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)
cards, warnings, _ = worker.extract(repository / 'examples/demo-paper', 'main.tex')
if warnings:
    raise RuntimeError('\n'.join(warnings))
for card in cards:
    card.pop('status', None)
    card['parentId'] = None
    card['sourceOrder'] *= 100

parent = next(card for card in cards if card['name'] == 'Trajectory')
customs = [
    ('Trajectory · slide', r's(t) = s_0 + v_0 t + \frac{1}{2}at^2', parent['id']),
    ('Gradient descent · slide', r'\mathbf{w}_{k+1} = \mathbf{w}_k - \eta\nabla\mathcal{L}(\mathbf{w}_k)', None),
    ('Rotation · slide', r'R(\theta)=\begin{pmatrix}\cos\theta&-\sin\theta\\\sin\theta&\cos\theta\end{pmatrix}', None),
]
for index, (name, tex, parent_id) in enumerate(customs):
    cards.append(dict(id=f'custom-demo-{index + 1}', kind='block', origin='custom',
                      name=name, tex=tex, sourceOrder=(len(cards) + 1) * 100,
                      parentId=parent_id, file=None, line=None, endLine=None,
                      context=[], contextStart=None))
(prototype / 'sample-data.js').write_text(
    '// Generated only from examples/demo-paper by scripts/build-samples.py.\n'
    'window.SAMPLE_CARDS = ' + json.dumps(cards, ensure_ascii=False, indent=2) + ';\n')
print(f'Created {len(cards)} cards from the synthetic demo paper.')
