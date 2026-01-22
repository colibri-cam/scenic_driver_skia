#!/bin/bash
# Wrapper script to force clang++ to use Nerves sysroot for cross-compilation
# Keep C++ headers but filter C include paths that conflict

args=()
for arg in "$@"; do
  # Keep C++ include paths (needed for C++ compilation)
  # But skip C include paths that conflict with sysroot
  if [[ "$arg" == "-I/usr/aarch64-linux-gnu/include" ]] || \
     [[ "$arg" == "-I/usr/arm-linux-gnueabihf/include" ]] || \
     [[ "$arg" == "-I/usr/x86_64-linux-musl/include" ]]; then
    continue
  fi

  args+=("$arg")
done

# Execute clang++ with filtered arguments
exec clang++ "${args[@]}"
