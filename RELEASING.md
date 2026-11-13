# Releasing Krust

## One-time setup

### 1. Create the update signing key

Run this yourself and choose a strong password. Keep the key file and
password safe and backed up: if they are lost, installed copies can never
update again; if they leak, someone could push updates to every user.

```
npx tauri signer generate -w %USERPROFILE%\.tauri\krust.key
```

### 2. Store the key as secrets of the `release` environment

```
gh secret set TAURI_SIGNING_PRIVATE_KEY --env release --repo avias8/krust < %USERPROFILE%\.tauri\krust.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --env release --repo avias8/krust
```

The second command prompts for the password. In the repository's
**Settings → Environments → release**, consider adding yourself as a
required reviewer, so every release run waits for your approval.

### 3. Put the public key in the app

Copy the contents of `krust.key.pub` into `plugins.updater.pubkey` in
`src-tauri/tauri.conf.json` and commit it. The public key is meant to be
public. The release workflow refuses to run while it is empty.

## Each release

1. Bump `version` in `src-tauri/tauri.conf.json` (and `src-tauri/Cargo.toml`)
   and commit.
2. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
3. The Release workflow builds the installer, signs the update bundle,
   writes `latest.json` and `SHA256SUMS.txt`, records a provenance
   attestation, and creates a **draft** release.
4. Review the draft, then **Publish release**. Installed copies see the
   update on their next launch.

## Windows code signing (Authenticode)

Not set up yet. Unsigned installers trigger a SmartScreen warning the first
time people run them. Options:

- [SignPath Foundation](https://signpath.org): free certificates for open
  source projects, after an application.
- [Azure Trusted Signing](https://azure.microsoft.com/products/trusted-signing):
  about US$10 a month, needs identity validation.

Either can be added to the Release workflow through Tauri's
`bundle.windows.signCommand`.
