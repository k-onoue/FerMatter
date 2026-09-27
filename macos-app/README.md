# Mathlidelux — native app

- Native SwiftUI/AppKit app with local Python compiler workers.
- Development prototype; macOS 26+ / Apple Silicon.
- [User guides](../user-docs/README.md) · [Development docs](../dev-docs/README.md).

## Requirements

- [Runtime and compiler requirements](../user-docs/requirements.md).
- To build: Xcode with the macOS 26 SDK; [development setup](../dev-docs/development.md#setup).

## Build and run

```sh
./macos-app/scripts/build.sh
open macos-app/build/Mathlidelux.app
```

- Run from the repository root; output is ad hoc signed for local development.
- [Checks](../dev-docs/testing.md), [architecture](../dev-docs/architecture.md), [release requirements](../dev-docs/releasing.md).

## Start a project

- Existing folder/ZIP/main file: [Open Project](../user-docs/paper-to-slides.md).
- Pasted text, Load File, templates: [New Project](../user-docs/new-project.md).
- [Synthetic samples](../user-docs/README.md#try-a-sample).

## Use the app

- [Edit a Library variant](../user-docs/edit-equation.md): preserve the manuscript.
- [Edit project source](../user-docs/source-editor.md): write actual project files.
- [Export formats and sizes](../user-docs/export.md).
- [PDF navigation](../user-docs/navigate-paper.md).
- [Figures](../user-docs/figures.md), [shortcuts/appearance](../user-docs/preferences.md), [limits](../user-docs/limits.md).
- [Storage and compatibility](../dev-docs/reference/storage.md).
