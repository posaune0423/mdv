# Security

## Supported use and trust boundaries

Use mdv with documents and renderer executables you trust. The rich viewer is not a sandbox for hostile Markdown: its browser backend can load remote images and local assets, and Mermaid SVG output is not independently sanitized. Do not use rich mode for untrusted documents until the remaining renderer isolation work in [issue #13](https://github.com/posaune0423/mdv/issues/13) is resolved.

Mermaid runs an explicitly configured `MDV_MERMAID_CMD` or an installed `mmdc`; it does not download npm packages automatically. Keep these executables and `PATH` trusted. Use `--no-mermaid` to prevent external Mermaid execution, including in plain-text mode.

The `o` key opens the first document link, which may not be visible in the viewport. Only HTTP, HTTPS, and mailto targets are accepted. This is not a focused-link chooser, and allowed URLs are not guaranteed trustworthy.

The installers and updater fetch rolling binaries from this repository's `main` branch over HTTPS. They do not verify an independent signature or build attestation. Building from a reviewed commit avoids downloading those binaries but still requires trusting the Rust toolchain and dependencies; it does not sandbox document rendering.

Repository-local agent instructions must be reviewed before trusting a checkout. Personal agent permission settings are intentionally excluded from version control.

## Reporting

Use this repository's GitHub Security tab to report privately if private reporting is available. Otherwise open an issue with a minimal, non-sensitive description asking for a private reporting channel; do not post secrets, private documents, or a weaponized exploit. Include the source commit or `mdv --version`, OS, terminal, input mode, and whether Mermaid was enabled.

Security reports stay open until their verified findings are addressed or explicitly dispositioned. A partial fix does not establish that arbitrary untrusted Markdown is safe.
