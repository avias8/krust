# Security policy

## Reporting a vulnerability

Please report security problems privately through
[GitHub's private vulnerability reporting](https://github.com/avias8/krust/security/advisories/new),
not in a public issue. Include what you found, how to reproduce it and the
Krust version (shown in the settings window, <kbd>F1</kbd>).

You should get a reply within a week. Fixes ship as a new release, which
installed copies offer to install automatically.

## Supported versions

Only the latest release receives fixes.

## How releases are protected

- **Built in public CI.** Every installer is built by the
  [Release workflow](.github/workflows/release.yml) on GitHub-hosted
  runners from a tagged commit, never on a developer's PC.
- **Signed updates.** Krust's auto-updater checks a minisign signature on
  every update against a public key built into the app, and refuses
  anything that does not match. The private key exists only as a GitHub
  Actions secret.
- **Build provenance.** Each installer has a
  [SLSA build provenance attestation](https://github.com/avias8/krust/attestations),
  signed through Sigstore, linking it to the exact commit and workflow run
  that built it.
- **Checksums.** Each release lists SHA-256 hashes in `SHA256SUMS.txt`.
- **Scanning.** CodeQL scans the Rust code, the settings page and the
  workflows; cargo-deny checks dependencies against the RustSec advisory
  database and an allowed-licence list; Dependabot proposes dependency
  updates; OpenSSF Scorecard rates the project's practices. GitHub Actions
  are pinned to full commit hashes.

## Verifying a download

With the [GitHub CLI](https://cli.github.com), check that an installer was
built by this repository's release workflow:

```
gh attestation verify Krust_0.1.0_x64-setup.exe --repo avias8/krust
```

Or compare its hash with `SHA256SUMS.txt` from the same release:

```
certutil -hashfile Krust_0.1.0_x64-setup.exe SHA256
```

## Design notes

- The game page (krunker.io) is remote content with no access to Krust's
  native commands. Only the bundled settings window may call them; see
  `src-tauri/build.rs` and `src-tauri/capabilities/settings.json`.
- Krust does not inject scripts into the game page or alter the files it
  loads.
