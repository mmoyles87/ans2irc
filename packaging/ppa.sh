#!/bin/sh
# Build Ubuntu source packages of a release tag for the PPA, one per series,
# sign them with the host's gpg-agent and, with --upload, send them to
# Launchpad. Everything Debian-specific runs in an ubuntu container.
#
#   packaging/ppa.sh 0.1.0                 # build and sign into build/ppa
#   packaging/ppa.sh 0.1.0 --upload        # ...and dput them
#   SERIES="noble" packaging/ppa.sh 0.1.0  # only some series
#
# Needs docker, git, and the signing key in the host's gpg-agent.
set -eu

version=${1:?usage: ppa.sh VERSION [--upload]}
upload=${2:-}
series=${SERIES:-"jammy noble resolute stonking"}
key=${DEBSIGN_KEYID:-7746D1F8D7DA0259}
ppa=${PPA:-ppa:mmoyles87/ans2irc}
maintainer="Matt Moyles <matt@mattmoyles.com>"
root=$(git rev-parse --show-toplevel)
out=$root/build/ppa
image=ubuntu:24.04

mkdir -p "$out"
orig="ans2irc_$version.orig.tar.gz"
git -C "$root" archive --prefix="ans2irc-$version/" -o "$out/$orig" "v$version"

for s in $series; do
    rev="$version-1~${s}1"
    work="$out/work-$s"
    mkdir -p "$work"
    cp "$out/$orig" "$work/"
    tar xzf "$out/$orig" -C "$work"
    cp -r "$root/packaging/debian" "$work/ans2irc-$version/debian"
    {
        printf 'ans2irc (%s) %s; urgency=medium\n\n  * Release %s for %s.\n\n -- %s  %s\n\n' \
            "$rev" "$s" "$version" "$s" "$maintainer" "$(date -R)"
        cat "$root/packaging/debian/changelog"
    } > "$work/ans2irc-$version/debian/changelog"
    docker run --rm -v "$work:/b" -w "/b/ans2irc-$version" -e UID_GID="$(id -u):$(id -g)" "$image" sh -c '
        apt-get update -qq >/dev/null
        DEBIAN_FRONTEND=noninteractive apt-get install -y -qq dpkg-dev debhelper >/dev/null 2>&1
        dpkg-buildpackage -S -sa -d -us -uc >/b/build.log 2>&1 || { tail -20 /b/build.log; exit 1; }
        chown -R "$UID_GID" /b'
    mv "$work"/ans2irc_"$rev"* "$out/"
    echo "built $rev"
done

# sign in a container that talks to the host's gpg-agent through its extra
# socket: the private key never leaves the host
agent=$(gpgconf --list-dirs agent-extra-socket)
keys=$(mktemp -d)
gpg --export "$key" > "$keys/key.gpg"
docker run --rm -v "$out:/out" -v "$keys:/keys:ro" -v "$agent:/tmp/agent" \
    -e KEY="$key" -e UID_GID="$(id -u):$(id -g)" "$image" sh -c '
    apt-get update -qq >/dev/null
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq devscripts gnupg dput >/dev/null 2>&1
    export GNUPGHOME=/tmp/gnupg && mkdir -m 700 $GNUPGHOME
    gpg -q --import /keys/key.gpg && ln -s /tmp/agent $GNUPGHOME/S.gpg-agent
    echo "no-autostart" > $GNUPGHOME/gpg.conf
    cd /out && debsign -k"$KEY" --re-sign *_source.changes
    chown -R "$UID_GID" /out'
rm -r "$keys"

if [ "$upload" = "--upload" ]; then
    docker run --rm -v "$out:/out" "$image" sh -c '
        apt-get update -qq >/dev/null
        DEBIAN_FRONTEND=noninteractive apt-get install -y -qq dput >/dev/null 2>&1
        cd /out && for c in *_source.changes; do dput '"$ppa"' "$c"; done'
fi
