# Changesets

This repo uses [Changesets](https://github.com/changesets/changesets) so every user-facing change carries a changelog entry and releases stay predictable.

## Maintainer flow

1. Install the [Changeset bot](https://github.com/apps/changeset-bot) on this repository (or add changeset files locally with `npx changeset`).
2. Open a PR; the bot can open a companion changeset if you forgot one.
3. Merge PRs that include `.changeset/*.md` files.
4. The **Changesets** GitHub Action opens a **Version Packages** PR (`package.json`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`).
5. Merge that PR; the action runs `changeset publish`, creates the `mdv@x.y.z` git tag, and the **Release** workflow uploads binaries to GitHub Releases.

## Local commands

```bash
npm ci
npx changeset   # add a changeset file
```

See the [Changesets documentation](https://github.com/changesets/changesets/blob/main/docs/intro-to-using-changesets.md) for details.
