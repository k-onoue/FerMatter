---
title: "macOS download notes"
description: "Signing, opening approval and checksum verification for the current macOS download."
---

## Opening on macOS

If macOS blocks a trusted copy from the official download, try opening it once, then follow [Apple’s opening instructions](https://support.apple.com/102445) to approve that app in **System Settings → Privacy & Security**.

## Verify the downloaded file

Run the command below in Terminal. If you saved the DMG somewhere else, adjust the path. Compare the result with the expected SHA-256 shown below.

A matching checksum confirms that the file matches the reviewed download. It does not establish Apple notarization or verify that the app works on your Mac.
