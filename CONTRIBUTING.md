# Contributing to glogout

Thanks for your interest in glogout! A few conventions keep the project tidy.

## Branch → PR → merge

**`main` is protected — no direct pushes.** All changes, including the
maintainer's, go through a pull request:

```bash
git switch -c my-change      # branch off main
# ...make your changes, commit them...
git push -u origin my-change
gh pr create                 # or open the PR in the GitHub UI
```

A PR can be merged into `main` once the **`build (release)`** CI check passes.
This applies to solo work too — the PR is the review checkpoint and keeps the
public history clean.

## Building locally

```bash
cargo build --release
```

You'll need the system libraries listed under **Requirements** in the
[README](README.md): GTK 4.14+, `gtk4-layer-shell`, and WebKitGTK 6.0.

## Scope

glogout targets Wayland compositors implementing `wlr-layer-shell` (validated on
Hyprland). See the README for supported-compositor details and known
limitations before filing a bug.
