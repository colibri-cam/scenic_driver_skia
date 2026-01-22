#!/bin/bash

#
# Build the cross-compilation sysroot for Scenic Driver Skia.
#
# This script:
# 1. Initializes Buildroot with the specified defconfig
# 2. Builds all required libraries (Mesa, libdrm, fontconfig, etc.)
# 3. Packages the sysroot for use with Rust cross-compilation
#
# Usage: ./build.sh <defconfig>
# Example: ./build.sh configs/scenic_skia_aarch64_defconfig
#

set -e

# Get the directory of this script
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$SCRIPT_DIR"

config="$1"

if [[ -z "$config" ]]; then
    echo "Usage: $0 <defconfig>"
    echo ""
    echo "Available configurations:"
    for cfg in configs/*_defconfig; do
        echo "  $cfg"
    done
    exit 1
fi

echo "=========================================="
echo "Building Scenic Driver Skia sysroot"
echo "Configuration: $config"
echo "=========================================="

# Initialize the build directory
echo ""
echo "Step 1: Initializing Buildroot..."
./create-build.sh "$config"
if [[ $? != 0 ]]; then
    echo "--> './create-build.sh $config' failed!"
    exit 1
fi

base=$(basename -s _defconfig "$config")
BUILD_DIR="o/$base"

echo ""
echo "Step 2: Building sysroot (this may take a while)..."
make -C "$BUILD_DIR" -j$(nproc)
if [[ $? != 0 ]]; then
    echo "--> Building sysroot failed!"
    exit 1
fi

echo ""
echo "Step 3: Packaging sysroot..."

# Create output directory
OUTPUT_DIR="sysroot"
mkdir -p "$OUTPUT_DIR"

# Determine the target architecture from the config name
case "$base" in
    *aarch64*)
        ARCH="aarch64"
        RUST_TARGET="aarch64-unknown-linux-gnu"
        ;;
    *armv7*)
        ARCH="armv7"
        RUST_TARGET="armv7-unknown-linux-gnueabihf"
        ;;
    *armv6*)
        ARCH="arm"
        RUST_TARGET="arm-unknown-linux-gnueabihf"
        ;;
    *x86_64*)
        ARCH="x86_64"
        RUST_TARGET="x86_64-unknown-linux-gnu"
        ;;
    *)
        ARCH="unknown"
        RUST_TARGET="unknown"
        echo "Warning: Unknown architecture from config name: $base"
        ;;
esac

SYSROOT_DIR="$OUTPUT_DIR/$ARCH"
SYSROOT_TARBALL="$OUTPUT_DIR/scenic-skia-sysroot-$ARCH.tar.gz"

# The staging directory contains headers and libraries for cross-compilation
# This is what we need for Rust/Skia
STAGING_DIR="$BUILD_DIR/staging"

# Clean previous sysroot
rm -rf "$SYSROOT_DIR"
mkdir -p "$SYSROOT_DIR"

echo "Copying staging directory to $SYSROOT_DIR..."
cp -a "$STAGING_DIR/"* "$SYSROOT_DIR/"

# Also copy some things from host that might be needed
HOST_DIR="$BUILD_DIR/host"

# Create the sysroot tarball
echo "Creating tarball: $SYSROOT_TARBALL..."
tar czf "$SYSROOT_TARBALL" -C "$OUTPUT_DIR" "$ARCH"

# Generate a helper script for setting up the cross-compilation environment
ENV_SCRIPT="$OUTPUT_DIR/env-$ARCH.sh"
TOOLCHAIN_DIR="$BUILD_DIR/host"
TOOLCHAIN_PREFIX=$(cat "$BUILD_DIR/.config" | grep ^BR2_TOOLCHAIN_EXTERNAL_CUSTOM_PREFIX | cut -d'"' -f2)

cat > "$ENV_SCRIPT" << EOF
#!/bin/bash
# Source this file to set up the cross-compilation environment
# Usage: source $ENV_SCRIPT

export SCENIC_SKIA_SYSROOT="$SCRIPT_DIR/$SYSROOT_DIR"
export SCENIC_SKIA_TARGET="$RUST_TARGET"

# Skia GN args for cross-compilation
export SKIA_GN_ARGS='
  target_os="linux"
  target_cpu="$ARCH"
  is_official_build=true
  cc="clang"
  cxx="clang++"
  extra_cflags=["--target=$RUST_TARGET","--sysroot='"\$SCENIC_SKIA_SYSROOT"'"]
  extra_cflags_cc=["--target=$RUST_TARGET","--sysroot='"\$SCENIC_SKIA_SYSROOT"'"]
  extra_ldflags=["--target=$RUST_TARGET","--sysroot='"\$SCENIC_SKIA_SYSROOT"'"]
'

# Cargo configuration for cross-compilation
export CARGO_TARGET_$(echo "$RUST_TARGET" | tr '[:lower:]-' '[:upper:]_')_LINKER="${TOOLCHAIN_PREFIX}gcc"

# pkg-config for finding libraries in sysroot
export PKG_CONFIG_SYSROOT_DIR="\$SCENIC_SKIA_SYSROOT"
export PKG_CONFIG_PATH="\$SCENIC_SKIA_SYSROOT/usr/lib/pkgconfig:\$SCENIC_SKIA_SYSROOT/usr/share/pkgconfig"
export PKG_CONFIG_ALLOW_CROSS=1

echo "Cross-compilation environment configured for $RUST_TARGET"
echo "Sysroot: \$SCENIC_SKIA_SYSROOT"
EOF

chmod +x "$ENV_SCRIPT"

# Collect license information
echo ""
echo "Step 4: Collecting license information..."
make -C "$BUILD_DIR" legal-info 2>/dev/null || true
if [[ -d "$BUILD_DIR/legal-info/licenses" ]]; then
    cp -r "$BUILD_DIR/legal-info/licenses" "$OUTPUT_DIR/licenses-$ARCH"
    echo "Licenses saved to $OUTPUT_DIR/licenses-$ARCH"
fi

echo ""
echo "Step 5: Extracting runtime libraries..."

# Create priv directory structure for runtime libraries
PRIV_DIR="../priv"
PRIV_LIB="$PRIV_DIR/lib/$ARCH"
PRIV_LIB_GBM="$PRIV_LIB/gbm"
mkdir -p "$PRIV_LIB"
mkdir -p "$PRIV_LIB_GBM"
mkdir -p "$PRIV_DIR/xkb"
mkdir -p "$PRIV_DIR/libinput-quirks"

TARGET_DIR="$BUILD_DIR/target"

# Copy runtime libraries needed on the device
# Graphics stack
cp -a "$TARGET_DIR/usr/lib/libdrm.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libEGL.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libGLESv2.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libgbm.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libglapi.so"* "$PRIV_LIB/" 2>/dev/null || true

# Mesa drivers
cp -a "$TARGET_DIR/usr/lib/libgallium"* "$PRIV_LIB/" 2>/dev/null || true
if [[ -d "$TARGET_DIR/usr/lib/gbm" ]]; then
    cp -a "$TARGET_DIR/usr/lib/gbm/"* "$PRIV_LIB_GBM/" 2>/dev/null || true
fi
if [[ -d "$TARGET_DIR/usr/lib/dri" ]]; then
    mkdir -p "$PRIV_LIB/dri"
    cp -a "$TARGET_DIR/usr/lib/dri/"* "$PRIV_LIB/dri/" 2>/dev/null || true
fi

# Wayland
cp -a "$TARGET_DIR/usr/lib/libwayland-client.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libwayland-server.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libwayland-egl.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libwayland-cursor.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libffi.so"* "$PRIV_LIB/" 2>/dev/null || true

# Input handling
cp -a "$TARGET_DIR/usr/lib/libinput.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libevdev.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libmtdev.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libxkbcommon.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/lib/libudev.so"* "$PRIV_LIB/" 2>/dev/null || true

# Font rendering
cp -a "$TARGET_DIR/usr/lib/libfontconfig.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libfreetype.so"* "$PRIV_LIB/" 2>/dev/null || true

# Image codecs
cp -a "$TARGET_DIR/usr/lib/libpng"*.so* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libjpeg.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libwebp.so"* "$PRIV_LIB/" 2>/dev/null || true

# Core libraries
cp -a "$TARGET_DIR/usr/lib/libexpat.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libz.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/lib/libdl.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/lib/libblkid.so"* "$PRIV_LIB/" 2>/dev/null || true
cp -a "$TARGET_DIR/usr/lib/libkmod.so"* "$PRIV_LIB/" 2>/dev/null || true

# ICU libraries (for text)
cp -a "$TARGET_DIR/usr/lib/libicu"*.so* "$PRIV_LIB/" 2>/dev/null || true

# XKB keyboard configuration
if [[ -d "$TARGET_DIR/usr/share/X11/xkb" ]]; then
    cp -a "$TARGET_DIR/usr/share/X11/xkb/"* "$PRIV_DIR/xkb/" 2>/dev/null || true
fi

# libinput quirks
if [[ -d "$TARGET_DIR/usr/share/libinput" ]]; then
    cp -a "$TARGET_DIR/usr/share/libinput/"*.quirks "$PRIV_DIR/libinput-quirks/" 2>/dev/null || true
fi

# Create a hash file to track which build produced these libs
echo "$ARCH-$(date +%Y%m%d-%H%M%S)" > "$PRIV_LIB/.build_hash"

echo "Runtime libraries copied to $PRIV_LIB"

echo ""
echo "Step 6: Patching RPATH on runtime libraries..."

# Use patchelf to set RPATH on all .so files so they find each other
# $ORIGIN expands to the directory containing the library at runtime
PATCHELF="$BUILD_DIR/host/bin/patchelf"
if [[ ! -x "$PATCHELF" ]]; then
    # Try system patchelf
    PATCHELF=$(which patchelf 2>/dev/null || true)
fi

if [[ -x "$PATCHELF" ]]; then
    echo "Using patchelf: $PATCHELF"

    # Patch all .so files to look for dependencies in the same directory
    for lib in "$PRIV_LIB"/*.so*; do
        if [[ -f "$lib" && ! -L "$lib" ]]; then
            echo "  Patching: $(basename "$lib")"
            "$PATCHELF" --set-rpath '$ORIGIN' "$lib" 2>/dev/null || true
        fi
    done

    # Also patch libraries in subdirectories (gbm, dri)
    for subdir in gbm dri; do
        if [[ -d "$PRIV_LIB/$subdir" ]]; then
            for lib in "$PRIV_LIB/$subdir"/*.so*; do
                if [[ -f "$lib" && ! -L "$lib" ]]; then
                    echo "  Patching: $subdir/$(basename "$lib")"
                    # These need to find libs in parent directory
                    "$PATCHELF" --set-rpath '$ORIGIN/..' "$lib" 2>/dev/null || true
                fi
            done
        fi
    done

    echo "RPATH patching complete"
else
    echo "WARNING: patchelf not found - libraries may not find their dependencies"
    echo "Install patchelf or add BR2_PACKAGE_HOST_PATCHELF=y to defconfig"
fi

echo ""
ls -la "$PRIV_LIB" | head -20

echo ""
echo "=========================================="
echo "Build complete!"
echo "=========================================="
echo ""
echo "Sysroot directory: $SYSROOT_DIR"
echo "Sysroot tarball:   $SYSROOT_TARBALL"
echo "Environment script: $ENV_SCRIPT"
echo "Runtime libraries: $PRIV_LIB"
echo ""
echo "To use with Rust cross-compilation:"
echo "  source $ENV_SCRIPT"
echo ""
echo "Or set these environment variables manually:"
echo "  export SCENIC_SKIA_SYSROOT=$SCRIPT_DIR/$SYSROOT_DIR"
echo "  export SKIA_GN_ARGS='...'"
echo ""
