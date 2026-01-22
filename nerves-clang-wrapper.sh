#!/bin/bash
# Wrapper script to force clang to use Nerves sysroot for cross-compilation
# This filters out problematic system include paths

# Filter out system include paths that conflict with Nerves sysroot
args=()
skip_next=false
for arg in "$@"; do
  if [ "$skip_next" = true ]; then
    skip_next=false
    continue
  fi

  # Skip system include paths
  if [[ "$arg" == "-I/usr/aarch64-linux-gnu/"* ]] ||  \
     [[ "$arg" == "-I/usr/arm-linux-"* ]] || \
     [[ "$arg" == "-I/usr/x86_64-linux-"* ]]; then
    continue
  fi

  args+=("$arg")
done

# Execute clang with filtered arguments
exec clang "${args[@]}"
