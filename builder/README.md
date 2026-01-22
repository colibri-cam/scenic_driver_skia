# Scenic Driver Skia - Cross-Compilation Builder

This directory contains a Buildroot-based system for creating cross-compilation sysroots for the scenic_driver_skia Rustler NIF.

## Overview

The builder creates a sysroot with all dependencies needed for cross-compiling:
- **Graphics**: Mesa3D (OpenGL ES, EGL, GBM), libdrm
- **Fonts**: fontconfig, freetype
- **Images**: libpng, jpeg, webp
- **Input**: libinput, libevdev, eudev, libxkbcommon
- **Text**: ICU (Unicode support)

The `rust-skia` crate builds Skia from source - this builder only provides the sysroot with dependent libraries.

## Requirements

- Linux x86_64 host
- Basic build tools: `make`, `gcc`, `g++`, `wget`, `tar`
- Clang/LLVM (for Skia cross-compilation)
- Rust with cross-compilation support
- ~10GB disk space for full build

## Quick Start

### 1. Build the sysroot

```bash
cd builder
./build.sh configs/scenic_skia_aarch64_defconfig
```

This will:
1. Download Buildroot
2. Download the Nerves toolchain
3. Build all required libraries
4. Package the sysroot in `sysroot/aarch64/`

### 2. Cross-compile the NIF

```bash
./cross-compile.sh aarch64 build --release
```

Or manually set up the environment:

```bash
source sysroot/env-aarch64.sh
cd ../native/scenic_driver_skia
cargo build --release --target aarch64-unknown-linux-gnu
```

## Available Configurations

| Config | Target | Description |
|--------|--------|-------------|
| `scenic_skia_aarch64_defconfig` | aarch64 | Raspberry Pi 4/5, other ARM64 devices |

## Directory Structure

```
builder/
├── build.sh                 # Main build script
├── create-build.sh          # Buildroot initialization
├── cross-compile.sh         # Helper for Rust cross-compilation
├── configs/
│   └── scenic_skia_aarch64_defconfig
├── scripts/
│   ├── download-buildroot.sh
│   └── buildroot-state.sh
├── br_patches/              # Patches for Buildroot (if needed)
├── patches/                 # Patches for packages (if needed)
├── o/                       # Build output directories
│   └── scenic_skia_aarch64/
│       ├── staging/         # Sysroot with headers + libs
│       ├── target/          # Target filesystem
│       └── host/            # Host tools + toolchain
└── sysroot/                 # Packaged sysroots for cross-compilation
    └── aarch64/
```

## How It Works

### Buildroot Configuration

The defconfig uses:
- **External Nerves toolchain**: Pre-built GCC 13.x cross-compiler
- **Standard packages**: Mesa3D, libdrm, fontconfig, etc.
- **Staging directory**: Contains headers and libraries for cross-compilation

### Skia Cross-Compilation

`rust-skia` builds Skia from source using GN/Ninja. The builder provides:

1. **SKIA_GN_ARGS**: Skia build configuration
   ```
   target_os="linux"
   target_cpu="arm64"
   cc="clang"
   cxx="clang++"
   extra_cflags=["--target=aarch64-unknown-linux-gnu","--sysroot=/path/to/sysroot"]
   ```

2. **Sysroot**: All required headers and libraries
3. **pkg-config**: Configured to find libraries in the sysroot

### Cargo Configuration

The `cross-compile.sh` script sets:
- `CARGO_TARGET_*_LINKER`: Cross-compiler or clang
- `CC`, `CXX`: clang with target flags
- `PKG_CONFIG_*`: For finding sysroot libraries

## Nerves Integration

For Nerves deployment, the precompiled `.so` file can be included in your firmware:

1. Build the sysroot
2. Cross-compile the NIF
3. Copy to your Nerves project's `priv/` directory

The mix.exs in the main project already handles Nerves cross-compilation when `NERVES_SDK_SYSROOT` is set.

## Customization

### Adding packages

1. Edit the defconfig to add Buildroot packages
2. Rebuild: `./build.sh configs/scenic_skia_aarch64_defconfig`

### Adding new architectures

1. Copy an existing defconfig
2. Update toolchain settings for the new arch
3. Add the new target to `cross-compile.sh`

## Troubleshooting

### Build fails downloading toolchain

Check your internet connection. The Nerves toolchain is downloaded from GitHub.

### Skia fails to compile

Ensure clang is installed and available. Skia requires clang for cross-compilation.

### pkg-config can't find libraries

Verify `PKG_CONFIG_SYSROOT_DIR` is set correctly:
```bash
source sysroot/env-aarch64.sh
pkg-config --libs drm
```

### Missing headers during compilation

The sysroot staging directory should contain all headers. If something is missing, add the package to the defconfig and rebuild.

## License

The sysroot contains libraries under various open source licenses. Run `make legal-info` in the build directory to collect license information, or see `sysroot/licenses-<arch>/` after building.
