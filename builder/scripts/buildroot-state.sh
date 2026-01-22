#!/bin/bash

#
# Print information about the state of the Buildroot tree.
#
# Inputs:
#   $1   The Buildroot version
#   $2   The directory containing patches for Buildroot
#
# Outputs:
#   Text describing the Buildroot tree
#

set -e
export LC_ALL=C

BR_VERSION=$1
PATCHES_DIR=$2

usage() {
    echo "buildroot-state.sh <BR version> [patches directory]"
}

# Trust the passed in version
echo "Buildroot: $BR_VERSION"

# If patches directory exists, hash the patches
if [[ -n "$PATCHES_DIR" && -d "$PATCHES_DIR" ]]; then
    if ls "$PATCHES_DIR"/*.patch >/dev/null 2>&1; then
        echo "Patches:"
        for patch in "$PATCHES_DIR"/*.patch; do
            echo "  $(basename "$patch"): $(sha256sum "$patch" | cut -d' ' -f1)"
        done
    fi
fi
