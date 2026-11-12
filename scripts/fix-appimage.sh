#!/usr/bin/env bash
# Removes the bundled Wayland client libraries from the Tauri AppImage and
# repacks it.
#
# Tauri 1 bundles libwayland-* from the build system (Ubuntu 22.04). On newer
# distributions (e.g. Fedora) those old copies clash with the host's Mesa/EGL
# drivers and WebKit aborts with "Could not create default EGL display:
# EGL_BAD_PARAMETER", leaving a blank window. Every Wayland/GTK desktop ships
# libwayland itself, so the host copy is used instead.
#
# Usage: scripts/fix-appimage.sh [bundle/appimage directory]
# Set APPIMAGETOOL=/path/to/appimagetool to use an existing appimagetool
# instead of downloading it.
set -euo pipefail

BUNDLE_DIR="${1:-src-tauri/target/release/bundle/appimage}"
APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"

shopt -s nullglob
appimages=("$BUNDLE_DIR"/*.AppImage)
appdirs=("$BUNDLE_DIR"/*.AppDir)
if [ ${#appimages[@]} -ne 1 ] || [ ${#appdirs[@]} -ne 1 ]; then
  echo "error: expected exactly one .AppImage and one .AppDir in $BUNDLE_DIR" >&2
  exit 1
fi
appimage="${appimages[0]}"
appdir="${appdirs[0]}"

# Always repack, even if an earlier run already deleted the libraries from the
# AppDir but failed before writing the new AppImage.
removed=$(find "$appdir" -name 'libwayland-*.so*' -print -delete)
echo "removed from AppDir: ${removed:-(none left)}"

appimagetool="${APPIMAGETOOL:-}"
if [ -z "$appimagetool" ]; then
  tool_dir="$(mktemp -d)"
  trap 'rm -rf "$tool_dir"' EXIT
  appimagetool="$tool_dir/appimagetool"
  curl -fsSL -o "$appimagetool" "$APPIMAGETOOL_URL"
  chmod +x "$appimagetool"
fi

# appimagetool is itself an AppImage; this lets it run without FUSE
# (containers, CI runners).
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$appimagetool" --no-appstream "$appdir" "$appimage.new"
mv "$appimage.new" "$appimage"
chmod +x "$appimage"
echo "repacked $appimage"
