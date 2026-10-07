#!/bin/sh
# Creates a self-signed code signing identity. Binaries signed with it keep the
# same identity across rebuilds, so macOS remembers keychain authorizations.
set -eu

identity="${IOPOLE_SIGNING_IDENTITY:-iopole-dev}"
keychain="${IOPOLE_SIGNING_KEYCHAIN:-$HOME/Library/Keychains/login.keychain-db}"

if security find-identity -p codesigning "$keychain" | grep -q "\"$identity\""; then
    echo "Signing identity \"$identity\" already exists."
    exit 0
fi

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT
password=$(openssl rand -hex 16)

openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=$identity" \
    -addext "basicConstraints=critical,CA:false" \
    -addext "keyUsage=critical,digitalSignature" \
    -addext "extendedKeyUsage=critical,codeSigning" \
    -keyout "$workdir/key.pem" -out "$workdir/cert.pem" 2>/dev/null

# macOS only imports PKCS#12 files using the legacy algorithms.
openssl pkcs12 -export -inkey "$workdir/key.pem" -in "$workdir/cert.pem" \
    -certpbe PBE-SHA1-3DES -keypbe PBE-SHA1-3DES -macalg sha1 \
    -passout "pass:$password" -out "$workdir/identity.p12"

security import "$workdir/identity.p12" -k "$keychain" -P "$password" -T /usr/bin/codesign

echo "Signing identity \"$identity\" created."
