#!/bin/sh
# Signs a binary with the identity from create-signing-identity.sh. Does nothing
# when the identity is missing.
set -eu

identity="${IOPOLE_SIGNING_IDENTITY:-iopole-dev}"

if security find-identity -p codesigning | grep -q "\"$identity\""; then
    codesign --force --sign "$identity" --identifier iopole "$1" 2>/dev/null
fi
