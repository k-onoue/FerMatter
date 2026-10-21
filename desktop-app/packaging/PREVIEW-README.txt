FerMatter — public preview (not notarized)

This preview is ad hoc signed. It has no Developer ID signature or Apple
notarization. macOS may require explicit approval on first launch.

INSTALL

Quit FerMatter before replacing it. Open the DMG, drag FerMatter.app into
Applications, eject the DMG, then open FerMatter from Applications.

If macOS blocks the app, try opening it once, then use System Settings >
Privacy & Security > Open Anyway for FerMatter. Only approve a copy obtained
from the official download page. Managed Macs may prohibit this override.
https://support.apple.com/102445

Homebrew: brew install --cask fermatter/tap/fermatter

REQUIREMENTS

Apple Silicon Mac, macOS 14 or later (verified on macOS 26).
Install Typst for Typst/CeTZ projects, or TeX Live/MacTeX and the conversion
tools listed in the guide for LaTeX/TikZ. Choose tool paths in Settings.
Compilers are not bundled. No Python, Node.js, Rust or Xcode is needed to run
FerMatter. See https://fermatter.github.io/guide/requirements/

UPDATES AND DATA

Update with brew upgrade --cask fermatter/tap/fermatter, or download the next
DMG and replace the app after quitting. There is no in-app update downloader.
Projects, settings, drafts and Instant history are kept outside the app;
replacement/removal preserves them. The existing bundle identity and legacy
Slide Math data paths remain unchanged. A new signing identity may require
folder access to be approved again. Source mode edits real project files.

LICENSE

Copyright 2026 k-onoue. Licensed under Apache-2.0; LICENSE and NOTICE accompany
the app. Dependency notices are in Contents/Resources/licenses/ and themes/.

Website and downloads: https://fermatter.github.io/
