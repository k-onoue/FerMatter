# Local signing and macOS folder access

- macOS remembers **Files and Folders → Documents Folder** consent for the app's code identity, not for each Mathlidelux project.
- Ad hoc signatures identify a particular build by its code hash. Rebuilding can invalidate earlier consent and trigger another prompt.
- Use the same Apple Development certificate and bundle ID for successive local builds. This gives macOS a stable identity; revoked permissions, certificate/team changes and separately protected locations can still require approval.
- Selecting the default parent folder in **Settings → Projects → Choose…** grants access to that selected location through the system picker. It does not fix a changing code identity across builds.

## Configure this Mac once

1. In **Xcode → Settings → Apple Accounts**, select your Apple Account/team.
2. Open **Manage Certificates… → + → Apple Development**. Complete sign-in or Keychain prompts yourself.
3. List installed signing identities:

   ```sh
   security find-identity -v -p codesigning
   ```

4. Save the desired certificate's exact name or 40-character SHA-1 locally:

   ```sh
   mkdir -p desktop-app/build
   printf '%s\n' 'Apple Development: YOUR NAME (TEAM/CERT ID)' > desktop-app/build/signing-identity.txt
   ./desktop-app/scripts/build.sh
   ```

- `build/signing-identity.txt` is ignored by Git; it contains only a certificate reference. Private keys stay in Keychain.
- `APPLE_SIGNING_IDENTITY` overrides the file for one build. An unavailable configured identity stops the build before compilation; there is no silent ad hoc fallback.
- If Xcode created a certificate but `security find-identity -v -p codesigning` reports no valid identities, check the certificate chain in Keychain Access. Install a missing WWDR intermediate only from [Apple PKI](https://www.apple.com/certificateauthority/), matching the certificate's issuer; retain the system's default trust settings.
- With neither setting, the existing ad hoc build remains available. Explicit `APPLE_SIGNING_IDENTITY=-` selects ad hoc signing for prototype packaging; switching signatures can trigger consent again.
- Restart the new bundle and approve the initial Documents request if needed. Check two rebuilt versions with the same certificate before claiming that upgrade consent was verified.

## Inspect

```sh
codesign --verify --deep --strict desktop-app/src-tauri/target/release/bundle/macos/Mathlidelux.app
codesign -d -r- desktop-app/src-tauri/target/release/bundle/macos/Mathlidelux.app
```

- A default ad hoc requirement shows `cdhash`; a certificate-backed requirement includes the signing identity/anchor.
- Development signing is for local testing. Public downloads still require Developer ID signing and notarization; the [prototype packager](packaging.md) accepts only ad hoc bundles.
- Do not reset/edit TCC databases, remove file protection, or grant Full Disk Access to work around rebuild prompts.

## References

- [Apple DTS: ad hoc signing and TCC](https://developer.apple.com/forums/thread/125438).
- [Apple certificate types](https://developer.apple.com/help/account/create-certificates/certificates-overview).
- [Tauri macOS code signing](https://v2.tauri.app/distribute/sign/macos/).
