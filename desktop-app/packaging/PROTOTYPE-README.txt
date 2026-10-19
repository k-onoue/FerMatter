FerMatter — local development prototype

This is an ad hoc signed development app, not a public release. It has no
Developer ID signature or Apple notarization. The installer package and disk
image are unsigned. macOS may block a downloaded copy. Packaging verification
checks archive integrity and the app's ad hoc signature; it does not establish
developer identity or Gatekeeper approval.

REQUIREMENTS

Apple Silicon Mac; see this build's minimum macOS version above (14 or later; verified on macOS 26).
No Python, Node.js, Rust, Swift SDK, or Xcode runtime installation is needed.

For Typst projects: Typst CLI 0.14 or later.

For LaTeX projects: TeX Live (for example, MacTeX), including latexmk, the
project's TeX engine, preview.sty, pdfcrop, and synctex; pdf2svg and Poppler
(pdfinfo and pdftocairo). DVI projects also need dvips and Ghostscript (ps2pdf).
Install any additional packages and fonts required by your project.

Typst, TeX, conversion tools, and project fonts are NOT bundled or installed
by this distribution. Install the tools for the selected project format.
Tools are searched in /Library/TeX/texbin, /opt/homebrew/bin, /usr/local/bin,
and the system command paths.

INSTALLATION — CHOOSE ONE

Quit FerMatter and save your work before replacing an existing app.

DMG: Open the disk image, drag FerMatter.app onto the Applications shortcut,
then eject the image and open FerMatter from Applications. Finder may request
permission to replace an existing app.

PKG: Open the installer and follow its steps. It installs only
/Applications/FerMatter.app, and may request administrator authorization.
It does not install runtimes, run installation scripts, or launch the app.
It replaces an existing app of the same identity; newer versions are preserved.

The app retains the prototype bundle identifier local.slidemath.prototype so
existing preferences remain available. New project source folders default to
~/Documents/Mathlidelux. Projects opened from an existing folder stay in that
location. Project metadata and caches live in
~/Library/Application Support/Slide Math/Rust Preview (the existing internal
folder name is retained). Legacy Swift data is kept in its original location;
missing projects/settings and recoverable source drafts are imported.
Source mode directly edits opened
project files; open only projects you trust.

If macOS blocks this unnotarized prototype, consult Apple's security guidance:
https://support.apple.com/102445
No security settings are changed by the packaging script or installer.

UPDATES

This version does not include an in-app update downloader. Download the next
build manually and replace the app. No background helper is installed.

REMOVAL

Quit the app and move /Applications/FerMatter.app to the Trash. Your projects,
templates, settings, and package receipt are retained.

LICENSE

FerMatter is licensed under Apache-2.0. LICENSE and NOTICE are included next
to the app on the disk image and inside the app's Contents/Resources folder.
The PKG installer includes the same files and displays the license.

Project source and build instructions: https://github.com/k-onoue/mathlidelux

Upgrading from Mathlidelux: quit the old app before opening FerMatter.
After installing FerMatter.app, remove only the old Mathlidelux.app bundle.
Keep your project folders and Application Support data; FerMatter reuses them.
