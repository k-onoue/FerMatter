"""Source-order document headings, shared by the LaTeX and Typst scanners."""
import collections
import hashlib
import re


class Outline:
    def __init__(self, headings=None):
        self.headings = headings if headings is not None else []
        self.stack = []
        self.occurrences = collections.Counter()

    @property
    def path(self):
        return [heading['id'] for heading in self.stack]

    def add(self, title, level, file, line):
        while self.stack and self.stack[-1]['level'] >= level:
            self.stack.pop()
        title = re.sub(r'\s+', ' ', title).strip() or 'Untitled'
        key = '\0'.join([file, str(level), title, *self.path])
        self.occurrences[key] += 1
        identifier = 'heading-' + hashlib.sha256((key + '\0' + str(self.occurrences[key])).encode()).hexdigest()[:24]
        heading = dict(id=identifier, title=title, level=level, file=file, line=line, path=self.path + [identifier])
        self.stack.append(heading)
        self.headings.append(heading)


def tex_title(text):
    text = re.sub(r'\\texorpdfstring\{[^{}]*\}\{([^{}]*)\}', r'\1', text)
    text = re.sub(r'\\(?:label|index)\{[^{}]*\}', '', text)
    text = re.sub(r'\\(?:protect|relax)\b', '', text)
    text = re.sub(r'\\([&%_$#{}])', r'\1', text)
    text = re.sub(r'\\[A-Za-z@]+\*?', '', text)
    return re.sub(r'[{}$]', '', text).replace('~', ' ')


def typst_title(text):
    text = re.sub(r'\s*<[^<>]+>\s*$', '', text.split('//', 1)[0])
    return re.sub(r'[*_`$]', '', text).strip()
