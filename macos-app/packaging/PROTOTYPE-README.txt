Mathlidelux — local development prototype

This is an ad hoc signed development app, not a public release. It has no
Developer ID signature or Apple notarization. The installer package and disk
image are unsigned. macOS may block a downloaded copy. Packaging verification
checks archive integrity and the app's ad hoc signature; it does not establish
developer identity or Gatekeeper approval.

REQUIREMENTS

Apple Silicon Mac; see this build's minimum macOS version above (26 or later).
Python 3 at /opt/homebrew/bin/python3, /usr/local/bin/python3, or /usr/bin/python3.

For Typst projects: Typst CLI 0.14 or later.

For LaTeX projects: TeX Live (for example, MacTeX), including latexmk, the
project's TeX engine, preview.sty, pdfcrop, and synctex; pdf2svg and Poppler
(pdfinfo and pdftocairo). DVI projects also need dvips and Ghostscript (ps2pdf).
Install any additional packages and fonts required by your project.

Python, Typst, TeX, conversion tools, and project fonts are NOT bundled or
installed by this distribution. Xcode is not needed to run the compiled app,
but a working Python 3 installation and the relevant project tools are needed.
Tools are searched in /Library/TeX/texbin, /opt/homebrew/bin, /usr/local/bin,
and the system command paths.

INSTALLATION — CHOOSE ONE

Quit Mathlidelux and save your work before replacing an existing app.

DMG: Open the disk image, drag Mathlidelux.app onto the Applications shortcut,
then eject the image and open Mathlidelux from Applications. Finder may request
permission to replace an existing app.

PKG: Open the installer and follow its steps. It installs only
/Applications/Mathlidelux.app, and may request administrator authorization.
It does not install runtimes, run installation scripts, or launch the app.
It replaces an existing app of the same identity; newer versions are preserved.

The app retains the prototype bundle identifier local.slidemath.prototype so
existing preferences remain available. New project source folders default to
~/Documents/Mathlidelux. Projects opened from an existing folder stay in that
location. Project metadata and caches live in
~/Library/Application Support/Slide Math. Source mode directly edits opened
project files; open only projects you trust.

If macOS blocks this unnotarized prototype, consult Apple's security guidance:
https://support.apple.com/102445
No security settings are changed by the packaging script or installer.

UPDATES

The app can check for new versions and download a release DMG after you accept
the update. Applying it requires you to replace the app manually. The app does
not replace itself automatically or install a background helper.

REMOVAL

Quit the app and move /Applications/Mathlidelux.app to the Trash. Your projects,
templates, settings, and package receipt are retained.

LICENSE

Mathlidelux is licensed under Apache-2.0. LICENSE and NOTICE are included next
to the app on the disk image and inside the app's Contents/Resources folder.
The PKG installer includes the same files and displays the license.

Project source and build instructions: https://github.com/k-onoue/mathlidelux
