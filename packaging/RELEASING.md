# Releasing

1. Set the new version in `Cargo.toml`, `doc/ans2irc.1` (the `.TH` line),
   `packaging/rpm/ans2irc.spec` (`Version`, and a `%changelog` entry),
   `packaging/debian/changelog` (a new entry on top) and both PKGBUILDs
   (`pkgver`, `pkgrel=1`). Update the version in the README's install links.
2. `cargo test`, commit, then tag and push: `git tag -s vX.Y.Z -m vX.Y.Z && git push origin main vX.Y.Z`.
   The release workflow builds the static binaries, `.deb`s and `.rpm`s for
   x86_64 and aarch64 and publishes them with `SHA256SUMS`; the macos workflow
   adds Apple silicon and Intel tarballs to the same release.
3. AUR, once the release is up: in each of `packaging/aur/ans2irc` and
   `packaging/aur/ans2irc-bin`, run `updpkgsums && makepkg --printsrcinfo > .SRCINFO`,
   build it with `makepkg -f`, then copy `PKGBUILD` and `.SRCINFO` into the
   AUR checkout (`git clone ssh://aur@aur.archlinux.org/<name>.git`), commit
   and push.
4. PPA: `packaging/ppa.sh X.Y.Z --upload` builds, signs and uploads a source
   package for each supported Ubuntu series. Launchpad builds them in an hour
   or so.
5. Homebrew: in `mmoyles87/homebrew-tap`, set `url` and `sha256` in
   `Formula/ans2irc.rb` to the new tag's tarball
   (`curl -sL https://github.com/mmoyles87/ans2irc/archive/refs/tags/vX.Y.Z.tar.gz | sha256sum`),
   then `brew install --build-from-source mmoyles87/tap/ans2irc && brew test ans2irc && brew audit --strict --online mmoyles87/tap/ans2irc`
   on a Mac, commit and push.
