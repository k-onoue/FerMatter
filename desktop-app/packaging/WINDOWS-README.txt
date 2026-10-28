FerMatter for Windows

This unsigned build targets Windows 11, x64. Windows 10, ARM64 and 32-bit
packages have not been validated.
Windows SmartScreen may identify an unsigned application. Verify the download
and SHA-256 checksum before choosing whether to run it. Keep Windows security
features enabled. This app has no code-signing certificate. See the accompanying build-info.json
for the release channel; a stable channel does not imply Authenticode signing.

The installer installs for the current user and includes Microsoft's offline
WebView2 installer. It does not require Node.js, Rust, Python or developer SDKs.
Compiler tools remain external: Typst, or a TeX distribution with latexmk,
pdfcrop and synctex plus Poppler (pdftocairo and pdfinfo).

Choose a project parent folder on first launch. Projects, drafts and application
data are stored outside the installation folder. Replacing or uninstalling
FerMatter preserves that data. New versions are installed manually; there is
no automatic updater.

LICENSE, NOTICE and licenses/ contain application and dependency notices.
See https://fermatter.github.io/ for setup instructions and known limitations.
