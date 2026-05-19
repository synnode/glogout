---
title: "AurPublishingPlan"
tags: [planning, packaging, aur, distribution]
related: ["DaemonMode"]
updated: 2026-05-19
---

# AurPublishingPlan

Plan for shipping `glogout` to the AUR. Not yet started — picked up in a follow-up session.

## Open prerequisites

These should be settled before submitting a package:

- **License.** README currently says `TBD`. AUR submission needs a real SPDX identifier. Most likely MIT or Apache-2.0 to match the Rust ecosystem default; needs an explicit decision.
- **Public repo + tagged release.** AUR `source=()` pulls from a tarball. Either tag a `v0.1.0` on GitHub and use the auto-generated archive, or commit to releasing tagged tarballs for every bump.
- **Crates.io publish?** Optional but useful — would let users skip the AUR entirely via `cargo install glogout`. Same versioning constraints either way.

## Packages to publish

Two AUR packages cover the typical preferences:

1. **`glogout`** — builds from source. PKGBUILD invokes `cargo build --release`, installs the binary, the systemd user unit from `contrib/`, and a stub `glogout init`-ready set of default configs into `/usr/share/glogout/`.
2. **`glogout-bin`** — prebuilt tarball from a GitHub release. Faster install, needed for users who don't want a full Rust toolchain.

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

## Steps for the next session

1. Decide on license, add `LICENSE` file, update `README.md`.
2. Tag `v0.1.0` on GitHub.
3. Write PKGBUILD + `.SRCINFO` for `glogout`.
4. Test in a clean Arch chroot via `extra-x86_64-build` or `makechrootpkg`.
5. Submit to AUR.
6. (Optional, same session if time allows) build `glogout-bin` PKGBUILD against the release tarball.

## Related
- [[DaemonMode]] — the systemd unit packaging needs to land alongside this
