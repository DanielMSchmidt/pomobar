#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script requires macOS (sips/iconutil/AppKit)." >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
output_icns="${1:-${repo_root}/resources/Pomobar.icns}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

base_png="${tmp_dir}/Pomobar-1024.png"
swift_file="${tmp_dir}/draw-icon.swift"
iconset_dir="${tmp_dir}/Pomobar.iconset"

cat > "${swift_file}" <<'SWIFT'
import AppKit

func color(_ hex: UInt32, alpha: CGFloat = 1.0) -> NSColor {
    let red = CGFloat((hex >> 16) & 0xFF) / 255
    let green = CGFloat((hex >> 8) & 0xFF) / 255
    let blue = CGFloat(hex & 0xFF) / 255
    return NSColor(srgbRed: red, green: green, blue: blue, alpha: alpha)
}

let outputPath = CommandLine.arguments[1]
let canvas = NSSize(width: 1024, height: 1024)

guard
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: Int(canvas.width),
        pixelsHigh: Int(canvas.height),
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    ),
    let context = NSGraphicsContext(bitmapImageRep: rep)
else {
    fatalError("Could not create bitmap context")
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = context

let fullRect = NSRect(origin: .zero, size: canvas)
NSColor.clear.setFill()
fullRect.fill()

let bgRect = NSRect(x: 64, y: 64, width: 896, height: 896)
let bgPath = NSBezierPath(roundedRect: bgRect, xRadius: 220, yRadius: 220)
let bgGradient = NSGradient(starting: color(0xF97316), ending: color(0xDC2626))!
bgGradient.draw(in: bgPath, angle: -45)

let tomatoRect = NSRect(x: 188, y: 170, width: 648, height: 648)
let tomatoPath = NSBezierPath(ovalIn: tomatoRect)
let tomatoGradient = NSGradient(starting: color(0xEF4444), ending: color(0xB91C1C))!
tomatoGradient.draw(in: tomatoPath, angle: -30)

let stemColor = color(0x166534)
for angle in [-38.0, -12.0, 12.0, 38.0] {
    let leaf = NSBezierPath(roundedRect: NSRect(x: 480, y: 700, width: 70, height: 190), xRadius: 35, yRadius: 35)
    var transform = AffineTransform.identity
    transform.translate(x: 512, y: 700)
    transform.rotate(byDegrees: angle)
    transform.translate(x: -512, y: -700)
    leaf.transform(using: transform)
    stemColor.setFill()
    leaf.fill()
}

let highlight = NSBezierPath(ovalIn: NSRect(x: 320, y: 500, width: 200, height: 120))
color(0xFFFFFF, alpha: 0.22).setFill()
highlight.fill()

let ring = NSBezierPath(ovalIn: NSRect(x: 330, y: 305, width: 364, height: 364))
ring.lineWidth = 26
color(0xFFFFFF, alpha: 0.88).setStroke()
ring.stroke()

let hand = NSBezierPath()
hand.move(to: NSPoint(x: 512, y: 487))
hand.line(to: NSPoint(x: 608, y: 590))
hand.lineWidth = 28
hand.lineCapStyle = .round
hand.lineJoinStyle = .round
color(0xFFFFFF, alpha: 0.94).setStroke()
hand.stroke()

let centerDot = NSBezierPath(ovalIn: NSRect(x: 486, y: 461, width: 52, height: 52))
color(0xFFFFFF, alpha: 0.98).setFill()
centerDot.fill()

NSGraphicsContext.restoreGraphicsState()

guard let png = rep.representation(using: .png, properties: [:]) else {
    fatalError("Could not encode PNG")
}

try png.write(to: URL(fileURLWithPath: outputPath))
SWIFT

swift "${swift_file}" "${base_png}"

mkdir -p "${iconset_dir}"
for size in 16 32 128 256 512; do
  sips -z "${size}" "${size}" "${base_png}" --out "${iconset_dir}/icon_${size}x${size}.png" >/dev/null
  retina=$((size * 2))
  sips -z "${retina}" "${retina}" "${base_png}" --out "${iconset_dir}/icon_${size}x${size}@2x.png" >/dev/null
done

iconutil -c icns "${iconset_dir}" -o "${output_icns}"
echo "Generated ${output_icns}"
