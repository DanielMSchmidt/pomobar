#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 4 ]]; then
  echo "Usage: $0 <binary-path> <version> <output-dir> [bundle-id]" >&2
  exit 1
fi

binary_path="$1"
version="$2"
output_dir="$3"
bundle_id="${4:-com.pomobar.Pomobar}"

if [[ ! -f "$binary_path" ]]; then
  echo "Binary not found: $binary_path" >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
icon_path="${repo_root}/resources/Pomobar.icns"

if [[ ! -f "$icon_path" ]]; then
  echo "Icon not found: $icon_path" >&2
  exit 1
fi

app_name="Pomobar"
app_bundle="${output_dir}/${app_name}.app"
contents_dir="${app_bundle}/Contents"
macos_dir="${contents_dir}/MacOS"
resources_dir="${contents_dir}/Resources"
exe_name="$(basename "$binary_path")"

rm -rf "$app_bundle"
mkdir -p "$macos_dir" "$resources_dir"

cp "$binary_path" "${macos_dir}/${exe_name}"
chmod +x "${macos_dir}/${exe_name}"
cp "$icon_path" "${resources_dir}/Pomobar.icns"

cat > "${contents_dir}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${app_name}</string>
  <key>CFBundleExecutable</key>
  <string>${exe_name}</string>
  <key>CFBundleIconFile</key>
  <string>Pomobar.icns</string>
  <key>CFBundleIdentifier</key>
  <string>${bundle_id}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${app_name}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${version}</string>
  <key>CFBundleVersion</key>
  <string>${version}</string>
  <key>LSMinimumSystemVersion</key>
  <string>10.15</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

echo "Created app bundle: ${app_bundle}"
