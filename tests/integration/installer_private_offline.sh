#!/bin/sh
# Hermetic Linux/GNU-utilities fixture; never invokes a genuine HyperMesh artifact.
set -eu
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$HERE/installer_private_offline/runner.py" "$@"
