#!/bin/bash

#
# Cross-compile scenic_driver_skia for a target architecture using the buildroot sysroot.
#
# This script sets up all necessary environment variables for:
# - Rust cross-compilation
# - Skia GN args for cross-building
# - pkg-config to find libraries in the sysroot
#
# Usage: ./cross-compile.sh <arch> [cargo arguments]
# Example: ./cross-compile.sh aarch64 build --release
#

set -e

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_DIR=$(dirname "$SCRIPT_DIR")

ARCH="$1"
shift || true

if [[ -z "$ARCH" ]]; then
    echo "Usage: $0 <arch> [cargo arguments]"
    echo ""
    echo "Available architectures:"
    for sysroot in "$SCRIPT_DIR"/sysroot/*/; do
        if [[ -d "$sysroot" ]]; then
            echo "  $(basename "$sysroot")"
        fi
    done
    echo ""
    echo "If no sysroot exists, build one first:"
    echo "  cd builder && ./build.sh configs/scenic_skia_<arch>_defconfig"
    exit 1
fi

SYSROOT_DIR="$SCRIPT_DIR/sysroot/$ARCH"

if [[ ! -d "$SYSROOT_DIR" ]]; then
    echo "ERROR: Sysroot not found: $SYSROOT_DIR"
    echo ""
    echo "Build the sysroot first:"
    echo "  cd builder && ./build.sh configs/scenic_skia_${ARCH}_defconfig"
    exit 1
fi

# Determine Rust target triple
case "$ARCH" in
    aarch64)
        RUST_TARGET="aarch64-unknown-linux-gnu"
        TARGET_CPU="arm64"
        ;;
    armv7)
        RUST_TARGET="armv7-unknown-linux-gnueabihf"
        TARGET_CPU="arm"
        ;;
    arm)
        RUST_TARGET="arm-unknown-linux-gnueabihf"
        TARGET_CPU="arm"
        ;;
    x86_64)
        RUST_TARGET="x86_64-unknown-linux-gnu"
        TARGET_CPU="x64"
        ;;
    *)
        echo "ERROR: Unknown architecture: $ARCH"
        exit 1
        ;;
esac

# Get toolchain info from the build
BUILD_DIR="$SCRIPT_DIR/o/scenic_skia_${ARCH}"
if [[ -f "$BUILD_DIR/.config" ]]; then
    TOOLCHAIN_PREFIX=$(grep ^BR2_TOOLCHAIN_EXTERNAL_CUSTOM_PREFIX "$BUILD_DIR/.config" | cut -d'"' -f2)
    TOOLCHAIN_BIN="$BUILD_DIR/host/bin"
else
    # Default to Nerves toolchain prefix
    case "$ARCH" in
        aarch64)
            TOOLCHAIN_PREFIX="aarch64-nerves-linux-gnu"
            ;;
        armv7)
            TOOLCHAIN_PREFIX="armv7-nerves-linux-gnueabihf"
            ;;
        arm)
            TOOLCHAIN_PREFIX="arm-nerves-linux-gnueabihf"
            ;;
        *)
            TOOLCHAIN_PREFIX="$ARCH-linux-gnu"
            ;;
    esac
    TOOLCHAIN_BIN=""
fi

# Uppercase target for Cargo env vars
CARGO_TARGET_UPPER=$(echo "$RUST_TARGET" | tr '[:lower:]-' '[:upper:]_')

echo "=========================================="
echo "Cross-compiling for $RUST_TARGET"
echo "Sysroot: $SYSROOT_DIR"
echo "=========================================="

# Export environment variables
export SCENIC_SKIA_SYSROOT="$SYSROOT_DIR"

# Create wrapper scripts for CC/CXX that include sysroot
# This ensures all C/C++ compilation uses our sysroot
WRAPPER_DIR="$SCRIPT_DIR/.wrappers"
mkdir -p "$WRAPPER_DIR"

# Find the GCC toolchain directory (contains C++ headers)
GCC_TOOLCHAIN="$BUILD_DIR/host/opt/ext-toolchain"
if [[ ! -d "$GCC_TOOLCHAIN" ]]; then
    echo "ERROR: GCC toolchain not found at $GCC_TOOLCHAIN"
    echo "Make sure you've run build.sh first"
    exit 1
fi

echo "GCC toolchain: $GCC_TOOLCHAIN"

# Find the GCC version for include paths
GCC_VERSION=$(ls "$GCC_TOOLCHAIN/aarch64-nerves-linux-gnu/include/c++/" | head -1)
GCC_TARGET="aarch64-nerves-linux-gnu"

echo "GCC version: $GCC_VERSION"

# C++ include paths - must be explicit because target triple differs
# Only include C++ standard library headers, NOT GCC's lib/gcc includes (arm_neon.h etc)
# as those are GCC-specific and incompatible with clang
CXX_INCLUDES="-isystem $GCC_TOOLCHAIN/$GCC_TARGET/include/c++/$GCC_VERSION"
CXX_INCLUDES="$CXX_INCLUDES -isystem $GCC_TOOLCHAIN/$GCC_TARGET/include/c++/$GCC_VERSION/$GCC_TARGET"
CXX_INCLUDES="$CXX_INCLUDES -isystem $GCC_TOOLCHAIN/$GCC_TARGET/include/c++/$GCC_VERSION/backward"
# Use clang's own intrinsics, not GCC's - do NOT add lib/gcc includes

# Clang needs explicit C++ include paths since target triple differs (nerves vs unknown)
# The wrapper also filters out system cross-compiler paths that rust-skia might add
cat > "$WRAPPER_DIR/clang-wrapper" << 'WRAPPER_EOF'
#!/bin/bash
args=()
skip_next=false
for arg in "$@"; do
    if $skip_next; then
        skip_next=false
        continue
    fi
    # Skip system cross-compiler include paths
    case "$arg" in
        -I/usr/aarch64-linux-gnu*|-I/usr/arm-linux-gnu*|-I/usr/x86_64-linux-gnu*)
            continue
            ;;
        --target=aarch64-linux-gnu|--target=arm-linux-gnu*)
            # Replace with our target
            continue
            ;;
    esac
    args+=("$arg")
done
WRAPPER_EOF
echo "exec clang --target=$RUST_TARGET --sysroot=$SYSROOT_DIR \"\${args[@]}\"" >> "$WRAPPER_DIR/clang-wrapper"
chmod +x "$WRAPPER_DIR/clang-wrapper"

cat > "$WRAPPER_DIR/clang++-wrapper" << 'WRAPPER_EOF'
#!/bin/bash
args=()
skip_next=false
for arg in "$@"; do
    if $skip_next; then
        skip_next=false
        continue
    fi
    # Skip system cross-compiler include paths
    case "$arg" in
        -I/usr/aarch64-linux-gnu*|-I/usr/arm-linux-gnu*|-I/usr/x86_64-linux-gnu*)
            continue
            ;;
        --target=aarch64-linux-gnu|--target=arm-linux-gnu*)
            # Replace with our target
            continue
            ;;
    esac
    args+=("$arg")
done
WRAPPER_EOF
# Add explicit C++ includes and other flags
echo "exec clang++ --target=$RUST_TARGET --sysroot=$SYSROOT_DIR $CXX_INCLUDES \"\${args[@]}\"" >> "$WRAPPER_DIR/clang++-wrapper"
chmod +x "$WRAPPER_DIR/clang++-wrapper"

# Target-specific CC/CXX for the cc crate (used by rust-skia)
TARGET_ENV=$(echo "$RUST_TARGET" | tr '-' '_')
export "CC_${TARGET_ENV}=$WRAPPER_DIR/clang-wrapper"
export "CXX_${TARGET_ENV}=$WRAPPER_DIR/clang++-wrapper"

# Use GCC toolchain's ar (llvm-ar may not be installed)
GCC_AR="$TOOLCHAIN_BIN/${TOOLCHAIN_PREFIX}-ar"
if [[ -x "$GCC_AR" ]]; then
    export "AR_${TARGET_ENV}=$GCC_AR"
    export AR="$GCC_AR"
else
    # Fallback to llvm-ar if available
    export "AR_${TARGET_ENV}=llvm-ar"
    export AR="llvm-ar"
fi

# Also set generic CC/CXX for other build scripts
export CC="$WRAPPER_DIR/clang-wrapper"
export CXX="$WRAPPER_DIR/clang++-wrapper"

# Skia GN args - only set non-conflicting options
# Don't set extra_cflags as rust-skia already sets it
# The CC/CXX wrappers handle the sysroot
export SKIA_GN_ARGS="target_os=\"linux\" target_cpu=\"$TARGET_CPU\""

# Cargo linker configuration - use the GCC toolchain linker
if [[ -n "$TOOLCHAIN_BIN" && -x "$TOOLCHAIN_BIN/${TOOLCHAIN_PREFIX}-gcc" ]]; then
    export "CARGO_TARGET_${CARGO_TARGET_UPPER}_LINKER=$TOOLCHAIN_BIN/${TOOLCHAIN_PREFIX}-gcc"
else
    # Fallback: use clang as linker with proper flags
    cat > "$WRAPPER_DIR/ld-wrapper" << EOF
#!/bin/bash
exec clang --target=$RUST_TARGET --sysroot=$SYSROOT_DIR -fuse-ld=lld "\$@"
EOF
    chmod +x "$WRAPPER_DIR/ld-wrapper"
    export "CARGO_TARGET_${CARGO_TARGET_UPPER}_LINKER=$WRAPPER_DIR/ld-wrapper"
fi

# Set RPATH so the NIF can find runtime libraries without LD_LIBRARY_PATH
# NIF is at: priv/native/libscenic_driver_skia.so
# Libs are at: priv/lib/<arch>/
# Relative path from NIF to libs: ../lib/<arch>
RPATH_FROM_NIF="\$ORIGIN/../lib/$ARCH"
export "CARGO_TARGET_${CARGO_TARGET_UPPER}_RUSTFLAGS=-C link-arg=-Wl,-rpath,$RPATH_FROM_NIF"

# pkg-config configuration
export PKG_CONFIG_SYSROOT_DIR="$SYSROOT_DIR"
export PKG_CONFIG_PATH="$SYSROOT_DIR/usr/lib/pkgconfig:$SYSROOT_DIR/usr/share/pkgconfig"
export PKG_CONFIG_ALLOW_CROSS=1

# Prevent system includes from being found
export CPATH=""
export C_INCLUDE_PATH=""
export CPLUS_INCLUDE_PATH=""

# Bindgen uses clang directly, not our wrapper - pass extra args
# Include the sysroot and C++ headers for cross-compilation
export BINDGEN_EXTRA_CLANG_ARGS="--target=$RUST_TARGET --sysroot=$SYSROOT_DIR $CXX_INCLUDES"

# Also set target-specific CXXFLAGS for the cc crate
export "CXXFLAGS_${TARGET_ENV}=--target=$RUST_TARGET --sysroot=$SYSROOT_DIR $CXX_INCLUDES"

# Add Rust target if not already installed
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
    echo "Installing Rust target: $RUST_TARGET"
    rustup target add "$RUST_TARGET"
fi

# Change to native directory and build
cd "$PROJECT_DIR/native/scenic_driver_skia"

if [[ $# -eq 0 ]]; then
    # Default to release build
    echo "Running: cargo build --release --target $RUST_TARGET"
    cargo build --release --target "$RUST_TARGET"
else
    echo "Running: cargo $@ --target $RUST_TARGET"
    cargo "$@" --target "$RUST_TARGET"
fi

echo ""
echo "=========================================="
echo "Build complete!"
echo "=========================================="
echo "Output: $PROJECT_DIR/native/scenic_driver_skia/target/$RUST_TARGET/"
