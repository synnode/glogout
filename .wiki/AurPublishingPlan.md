---
title: "AurPublishingPlan"
tags: [planning, packaging, aur, crates-io, distribution]
related: ["DaemonMode"]
updated: 2026-05-25
---

# AurPublishingPlan

Plan for shipping `glogout` publicly as part of the **1.0 release**. Distribution scope decided: **AUR + crates.io**.

## Prerequisites — status

- **License — DONE.** MIT chosen; `LICENSE` file committed and `license = "MIT"` is in `Cargo.toml`. (Was the original open question; now resolved.)
- **Public repo — DONE.** `git@github.com:synnode/glogout` exists and tags through `v0.3.0` are pushed. 1.0 adds a `v1.0.0` tag + GitHub release whose auto-generated tarball the AUR `source=()` pulls from.
- **crates.io publish — IN SCOPE for 1.0.** Lets users `cargo install glogout` (the README promises this). `Cargo.toml` now carries the required metadata (`description`, `repository`, `homepage`, `readme`, `keywords`, `categories`). Run `cargo publish` after the version bump lands.

## Packages to publish

Two AUR packages cover the typical preferences:

1. **`glogout`** — builds from source. PKGBUILD invokes `cargo build --release`, installs the binary, the systemd user unit from `contrib/`, and default configs into `/usr/share/glogout/`.
2. **`glogout-bin`** — prebuilt tarball from a GitHub release. Faster install, for users without a Rust toolchain.

`-git` package is overkill for now; the regular package off tagged releases is enough.

## PKGBUILD sketch (source package)

Critical bits to remember when writing it:

- `arch=('x86_64')` — webkit6 + gtk4-layer-shell aren't going to work cleanly elsewhere out of the box.
- `depends=('gtk4' 'gtk4-layer-shell' 'webkitgtk-6.0')`
- `makedepends=('rust' 'cargo')`
- `optdepends=('systemd: for the user-service unit')`
- Build: `cargo build --frozen --release --all-features` (use the lockfile for reproducibility).
- Package step installs:
  - `target/release/glogout` → `/usr/bin/glogout`
  - `contrib/glogout.service` → `/usr/lib/systemd/user/glogout.service`
  - `LICENSE` → `/usr/share/licenses/$pkgname/LICENSE`
  - `README.md` → `/usr/share/doc/$pkgname/README.md`

Use `install -Dm644 ...` for everything except the binary (which is `-Dm755`).

## Why not flatpak / nix / .deb yet

Out of scope. Nix flake would be a nice second package once AUR is up — the dependency closure is well-defined. .deb only matters if there's actual demand from Debian/Ubuntu users; layer-shell on those distros is patchy anyway.

## Remaining steps for the 1.0 release

1. Tag `v1.0.0` on GitHub + create the release (auto-tarball). *(outward-facing — confirm before pushing)*
2. `cargo publish` to crates.io. *(outward-facing — irreversible per version)*
3. Write PKGBUILD + `.SRCINFO` for `glogout`.
4. Test in a clean Arch chroot via `extra-x86_64-build` or `makechrootpkg`.
5. Submit to AUR.
6. (Optional, if time allows) build `glogout-bin` PKGBUILD against the release tarball.

## Related
- [[DaemonMode]] — the systemd unit packaging needs to land alongside this
