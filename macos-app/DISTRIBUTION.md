# Prototype distribution

`scripts/package.sh` creates a DMG and a macOS Installer PKG from the native
app. These are **local development prototypes**, not public release artifacts:
the app is ad hoc signed, the containers are unsigned, and none are notarized.
The artifact names include `prototype-unnotarized` to keep that distinction
visible when files are copied elsewhere.

## Build the artifacts

On an Apple Silicon Mac with macOS 26+ and Xcode 26+, from the repository root:

```sh
./macos-app/scripts/package.sh
```

This runs the existing `build.sh` and then packages its result. To package an
already built app, including one built in another checkout, without rebuilding:

```sh
./macos-app/scripts/package.sh --app "/path/to/Mathlidelux.app"
```

The input must be a valid ad hoc signed, arm64-only Mathlidelux app containing
`LICENSE` and `NOTICE`. Its bundle identifier must remain
`local.slidemath.prototype`. The script reads its version, build number, and
minimum macOS version from `Info.plist`; it does not edit or sign the input.
It rejects production-signed inputs because this is specifically a prototype
packaging workflow.

The default output directory is `macos-app/build/distribution/`, which Git
ignores. With version 0.10.0, build 10, the outputs are:

```text
Mathlidelux-0.10.0-build10-macOS-arm64-prototype-unnotarized.dmg
Mathlidelux-0.10.0-build10-macOS-arm64-prototype-unnotarized.pkg
Mathlidelux-0.10.0-build10-macOS-arm64-prototype-unnotarized.sha256
```

`--output "/path/to/directory"` changes the output directory; its parent must
already exist. Existing files are preserved unless `--overwrite` is passed.
The script stages and verifies both
archives before moving the three outputs into place. All temporary build and
inspection files are removed on exit; if a disk image cannot be detached, its
temporary directory is retained and reported for manual cleanup.

The build procedure is repeatable, but the archives are not promised to be
byte-for-byte reproducible: Apple's packaging tools include filesystem and
archive metadata. The SHA-256 file describes that particular packaging run:

```sh
cd macos-app/build/distribution
shasum -a 256 -c Mathlidelux-0.10.0-build10-macOS-arm64-prototype-unnotarized.sha256
```

## Contents and installation

The DMG contains `Mathlidelux.app`, an `Applications` symlink to `/Applications`,
and `README.txt`, `LICENSE`, and `NOTICE`. A user can drag the app onto the
Applications shortcut, eject the image, and launch the installed copy.

The PKG displays the same README and Apache-2.0 license. Its payload contains
only `/Applications/Mathlidelux.app`; license and notice files also remain
inside the signed bundle. The installer is restricted to the local system
domain, Apple Silicon, and the minimum macOS version declared by the app.
Bundle relocation is disabled, so an app in another folder is not selected as
the install destination. An existing matching bundle is replaced atomically,
with version checking enabled to preserve a newer installed version. Quit the
app and save work before installing. Installation may require administrator
authorization.

There are no preinstall/postinstall scripts, background services, automatic app
launches, security-setting changes, runtime installers, or updater setup. The
script only creates and inspects the packages; it does not install them.

The existing bundle identifier is preserved so preferences remain compatible.
User projects, templates, and settings under
`~/Library/Application Support/Slide Math` are not part of the installer.
Removing the app in Finder preserves those files and the installer receipt.

## External dependencies and release status

Python 3, Typst, TeX Live, conversion tools, and project-specific packages/fonts
are **not bundled**. Recipients must install a working Python 3 interpreter and
the tools for their project format. The exact requirements and supported search
paths are included in [the recipient README](packaging/PROTOTYPE-README.txt)
and [the native app guide](README.md#requirements). Xcode is required to build
from source, not to run the compiled app.

An ad hoc signature verifies bundle integrity without authenticating the
developer. Downloaded copies may be blocked by Gatekeeper. See
[Apple's guidance on safely opening apps](https://support.apple.com/102445).
This workflow does not remove quarantine attributes, disable Gatekeeper, access
signing credentials, or claim notarization. Installer verification is not a
clean-machine launch or install test.

Do not rename these artifacts to the public release asset `Mathlidelux.dmg`,
publish them as an automatic update, or set the website's `releaseAvailable`
flag to true. A public release still requires a separate dependency strategy,
Developer ID signing, notarization, and validation on a clean supported Mac.

## Verification performed by the script

- `codesign --verify --deep --strict` validates the input app, the staged copy,
  the app on the read-only mounted DMG, and the app expanded from the PKG.
- `hdiutil verify` checks DMG checksums. A read-only mount confirms the
  `/Applications` shortcut and exact README/LICENSE/NOTICE contents, then is
  detached.
- `pkgutil --expand-full` extracts the package without installing it. The
  extracted app is compared with the staged app; the package root, disabled
  relocation, and absence of installation scripts are checked.
- `pkgutil --check-signature` must report an unsigned package, matching the
  prototype designation. SHA-256 hashes are written for both final containers.

For manual inspection without installing, expand the final PKG into a new
temporary directory with `pkgutil --expand-full`, then inspect `Distribution`
and `Mathlidelux-component.pkg/PackageInfo`. The payload app is under
`Mathlidelux-component.pkg/Payload/Applications/Mathlidelux.app`.

The distribution uses Apple's native `hdiutil`, `pkgbuild`, and `productbuild`
tools; their installed manual pages document the command options. Installer
requirements and presentation use Apple's
[Distribution XML schema](https://developer.apple.com/library/archive/documentation/DeveloperTools/Reference/DistributionDefinitionRef/Chapters/Distribution_XML_Ref.html).
