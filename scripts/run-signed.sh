#!/bin/sh
set -eu
"$(dirname "$0")/sign.sh" "$1"
exec "$@"
