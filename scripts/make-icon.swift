// Draw the application icon of OpenCraft and write it as an `.icns` file.
//
// The icon is a dark rounded tile with 6 keys in a grid of 3 by 2 and one
// knob, as on a StreamDock-class controller. It has no logo of a vendor. The
// script draws each size from the same shapes, thus each size is sharp.
//
// Usage: swift scripts/make-icon.swift <output.icns> [<preview directory>]
//
// The script writes the PNG files into a temporary `.iconset` directory and
// then calls `iconutil`. With a preview directory, it also keeps a copy of the
// 1024, 256, 64, 32, and 16 pixel PNG files there.

import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

/// A colour from a hexadecimal RGB value.
func color(_ rgb: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(
        srgbRed: CGFloat((rgb >> 16) & 0xFF) / 255,
        green: CGFloat((rgb >> 8) & 0xFF) / 255,
        blue: CGFloat(rgb & 0xFF) / 255,
        alpha: alpha)
}

func gradient(_ top: UInt32, _ bottom: UInt32) -> CGGradient {
    CGGradient(
        colorsSpace: CGColorSpace(name: CGColorSpace.sRGB),
        colors: [color(top), color(bottom)] as CFArray,
        locations: [0, 1])!
}

/// Fill `path` with a vertical gradient from `top` to `bottom`.
func fill(_ context: CGContext, _ path: CGPath, _ top: UInt32, _ bottom: UInt32) {
    let box = path.boundingBox
    context.saveGState()
    context.addPath(path)
    context.clip()
    context.drawLinearGradient(
        gradient(top, bottom),
        start: CGPoint(x: box.midX, y: box.maxY),
        end: CGPoint(x: box.midX, y: box.minY),
        options: [])
    context.restoreGState()
}

func roundedRect(_ rect: CGRect, _ radius: CGFloat) -> CGPath {
    CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil)
}

/// The lit colour of each key face, top and bottom of the gradient. The first
/// row is first.
let keyColors: [(UInt32, UInt32)] = [
    (0x5EE7D9, 0x1FA89B),
    (0x6FA8FF, 0x3365E0),
    (0xB18CFF, 0x7347DB),
    (0xFFB561, 0xF07C1E),
    (0x6BE39B, 0x23A85C),
    (0xFF7A9F, 0xDB3767),
]

/// Draw the icon on a canvas of 1024 by 1024 units. The origin is at the
/// bottom left. The tile follows the grid of the macOS icons: 824 units with
/// a margin of 100 units.
func draw(_ context: CGContext) {
    let tile = CGRect(x: 100, y: 100, width: 824, height: 824)
    let tilePath = roundedRect(tile, 185)

    // The shadow below the tile.
    context.saveGState()
    context.setShadow(offset: CGSize(width: 0, height: -12), blur: 28, color: color(0x000000, 0.45))
    context.addPath(tilePath)
    context.setFillColor(color(0x1C1F25))
    context.fillPath()
    context.restoreGState()

    // The tile: the body of the device.
    fill(context, tilePath, 0x3B404B, 0x17191E)
    context.saveGState()
    context.addPath(roundedRect(tile.insetBy(dx: 3, dy: 3), 182))
    context.setStrokeColor(color(0xFFFFFF, 0.10))
    context.setLineWidth(6)
    context.strokePath()
    context.restoreGState()

    // The keys: 3 columns and 2 rows.
    let key: CGFloat = 176
    let gap: CGFloat = 40
    let left = tile.midX - (3 * key + 2 * gap) / 2
    let top: CGFloat = 832
    for row in 0..<2 {
        for column in 0..<3 {
            let x = left + CGFloat(column) * (key + gap)
            let y = top - key - CGFloat(row) * (key + gap)
            let rect = CGRect(x: x, y: y, width: key, height: key)
            // The frame of the key, a little darker than the tile.
            fill(context, roundedRect(rect, 38), 0x101216, 0x0B0C0F)
            // The lit face of the key.
            let (light, dark) = keyColors[row * 3 + column]
            let face = rect.insetBy(dx: 12, dy: 12)
            fill(context, roundedRect(face, 28), light, dark)
            // A soft gloss on the upper half of the face.
            let gloss = CGRect(x: face.minX, y: face.midY + 10, width: face.width, height: face.height / 2 - 10)
            context.saveGState()
            context.addPath(roundedRect(face, 28))
            context.clip()
            context.addPath(roundedRect(gloss.insetBy(dx: 10, dy: 0).offsetBy(dx: 0, dy: -8), 22))
            context.setFillColor(color(0xFFFFFF, 0.16))
            context.fillPath()
            context.restoreGState()
        }
    }

    // The knob, below the keys, in the centre.
    let center = CGPoint(x: tile.midX, y: 278)
    let radius: CGFloat = 92
    context.saveGState()
    context.setShadow(offset: CGSize(width: 0, height: -8), blur: 18, color: color(0x000000, 0.55))
    context.addEllipse(in: CGRect(x: center.x - radius, y: center.y - radius, width: 2 * radius, height: 2 * radius))
    context.setFillColor(color(0x0E1013))
    context.fillPath()
    context.restoreGState()
    let ring = radius - 10
    let knob = CGPath(
        ellipseIn: CGRect(x: center.x - ring, y: center.y - ring, width: 2 * ring, height: 2 * ring),
        transform: nil)
    fill(context, knob, 0x8A909C, 0x3A3F48)
    let cap = ring - 18
    let capPath = CGPath(
        ellipseIn: CGRect(x: center.x - cap, y: center.y - cap, width: 2 * cap, height: 2 * cap),
        transform: nil)
    fill(context, capPath, 0x5C626D, 0x2C3038)
    // The mark on the knob, to the upper right.
    let angle = CGFloat.pi / 4
    let inner = cap * 0.30
    let outer = cap * 0.82
    context.saveGState()
    context.setLineCap(.round)
    context.setLineWidth(20)
    context.setStrokeColor(color(0xF2F4F7))
    context.move(to: CGPoint(x: center.x + inner * cos(angle), y: center.y + inner * sin(angle)))
    context.addLine(to: CGPoint(x: center.x + outer * cos(angle), y: center.y + outer * sin(angle)))
    context.strokePath()
    context.restoreGState()
}

/// Draw the icon at `pixels` by `pixels`, and write it as a PNG file.
func writePNG(pixels: Int, to url: URL) {
    let context = CGContext(
        data: nil, width: pixels, height: pixels, bitsPerComponent: 8, bytesPerRow: 0,
        space: CGColorSpace(name: CGColorSpace.sRGB)!,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    context.interpolationQuality = .high
    context.setShouldAntialias(true)
    context.scaleBy(x: CGFloat(pixels) / 1024, y: CGFloat(pixels) / 1024)
    draw(context)
    let image = context.makeImage()!
    let destination = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)!
    CGImageDestinationAddImage(destination, image, nil)
    if !CGImageDestinationFinalize(destination) {
        fatalError("Cannot write \(url.path)")
    }
}

let arguments = CommandLine.arguments
guard arguments.count == 2 || arguments.count == 3 else {
    FileHandle.standardError.write("usage: swift scripts/make-icon.swift <output.icns> [<preview directory>]\n".data(using: .utf8)!)
    exit(2)
}
let output = URL(fileURLWithPath: arguments[1])
let files = FileManager.default
let work = files.temporaryDirectory.appendingPathComponent("opencraft-icon-\(ProcessInfo.processInfo.processIdentifier)")
let iconset = work.appendingPathComponent("OpenCraft.iconset")
try files.createDirectory(at: iconset, withIntermediateDirectories: true)
defer { try? files.removeItem(at: work) }

// The names and the sizes that `iconutil` expects.
for points in [16, 32, 128, 256, 512] {
    writePNG(pixels: points, to: iconset.appendingPathComponent("icon_\(points)x\(points).png"))
    writePNG(pixels: 2 * points, to: iconset.appendingPathComponent("icon_\(points)x\(points)@2x.png"))
}

if arguments.count == 3 {
    let preview = URL(fileURLWithPath: arguments[2])
    try files.createDirectory(at: preview, withIntermediateDirectories: true)
    for pixels in [1024, 256, 64, 32, 16] {
        writePNG(pixels: pixels, to: preview.appendingPathComponent("icon-\(pixels).png"))
    }
}

let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", iconset.path, "-o", output.path]
try iconutil.run()
iconutil.waitUntilExit()
if iconutil.terminationStatus != 0 {
    FileHandle.standardError.write("iconutil failed\n".data(using: .utf8)!)
    exit(1)
}
print("Wrote \(output.path)")
