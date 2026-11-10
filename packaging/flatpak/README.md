# Flatpak

A self-hosted Flatpak of QuickTalk. It isn't on Flathub. Tagged releases carry an
`io.github.gitguy268.quicktalk.flatpak` file, built by `.github/workflows/build.yml`.

Install it:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user ./io.github.gitguy268.quicktalk.flatpak
flatpak run io.github.gitguy268.quicktalk
```

Build it yourself on Ubuntu 24.04 (x86_64), from the repo root:

```sh
python3 scripts/build-engine.py && npm ci && npm run tauri -- build --bundles deb
cp src-tauri/target/release/bundle/deb/*.deb packaging/flatpak/quicktalk.deb
git clone https://github.com/flathub/shared-modules packaging/flatpak/shared-modules
cd packaging/flatpak
flatpak-builder --user --install-deps-from=flathub --force-clean --repo=repo build io.github.gitguy268.quicktalk.yml
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo repo quicktalk.flatpak io.github.gitguy268.quicktalk
```

Global shortcuts and typing into other apps only work under X11; Wayland doesn't allow them.
