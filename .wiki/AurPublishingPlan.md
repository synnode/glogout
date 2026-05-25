---
title: "AurPublishingPlan"
tags: [planning, packaging, aur, crates-io, distribution, release, runbook]
related: ["DaemonMode"]
updated: 2026-05-25
---

# AurPublishingPlan

`glogout` 1.0.0 shipped publicly on **2026-05-25** to both **crates.io** and the **AUR**. This page is now the release runbook for future versions.

## Status — 1.0.0 DONE

- **License:** MIT (`LICENSE` + `Cargo.toml`).
- **GitHub release:** tag `v1.0.0` + release at https://github.com/synnode/glogout/releases/tag/v1.0.0
- **crates.io:** published — `cargo install glogout` works. `Cargo.toml` carries the publish metadata and an `exclude` list that keeps `.wiki/`, `glogout_spec.md`, `.mcp.json`, CI, and editor dirs out of the registry tarball (lean 17-file crate).
- **AUR:** package `glogout` (source build), maintainer **Synnode**. PKGBUILD pulls the GitHub release tarball, builds with `cargo build --frozen --release`, installs binary + `contrib/glogout.service` + LICENSE + README. Validated with a local `makepkg` build (no clean chroot — `makechrootpkg`/devtools not installed on this machine).
- **`glogout-bin`:** not done — optional, skipped for 1.0.

## Release runbook (repeat for each new version)

1. Bump `version` in `Cargo.toml`, rebuild to sync `Cargo.lock`, commit.
2. `git push origin main`; tag `vX.Y.Z` (annotated) and push it. *Add any `Cargo.toml` packaging tweaks before tagging so tag == published crate.*
3. `gh release create vX.Y.Z --repo synnode/glogout --verify-tag ...`
4. `cargo publish --dry-run` → check `cargo package --list` is lean → `cargo publish`. **Irreversible per version** (can only yank).
5. AUR: in `/tmp/aur-glogout` (or a fresh clone of `ssh://aur@aur.archlinux.org/glogout.git`), update `pkgver`, run `updpkgsums`, `makepkg --printsrcinfo > .SRCINFO`, commit PKGBUILD + .SRCINFO, `git push origin master` (AUR uses the **master** branch).

## SSH gotcha for the AUR push (cost real time on 1.0)

- AUR auth uses the **`id_rsa`** key (passphrase-protected); GitHub uses the passphraseless `id_ed25519`. `ssh -T aur@aur.archlinux.org` greets with **"Welcome to AUR, Synnode!"** when it works.
- The agent must hold `id_rsa` **and the agent must be the one Claude's Bash tool sees.** Running `ssh-add` in a separate terminal does *not* share with the tool environment; running it **in-session via the `!` prefix** (`! ssh-add ~/.ssh/id_rsa`) does. Verify with `ssh-add -l` from a tool Bash call before pushing.
- Do *not* add a passphraseless key to AUR just to automate — keep `id_rsa` and load it per session.

## PKGBUILD essentials (kept in `/tmp/aur-glogout`)

- `arch=('x86_64')`, `depends=('gtk4' 'gtk4-layer-shell' 'webkitgtk-6.0')`, `makedepends=('cargo')`, `optdepends=('systemd: ...')`.
- `source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")`.
- Install: binary `-Dm755` → `/usr/bin`; service/LICENSE/README `-Dm644` → systemd user dir / licenses / doc.

## Not yet / out of scope

- `glogout-bin` (prebuilt tarball package) — optional follow-up.
- Nix flake, flatpak, .deb — out of scope; revisit only on demand.

## Related
- [[DaemonMode]] — the systemd unit packaging ships alongside this
