import CoreGraphics
import Foundation
import ImageIO

guard CommandLine.arguments.count == 2 else {
    fatalError("Usage: dmg-background.swift OUTPUT_PREFIX")
}

let prefix = CommandLine.arguments[1]
let points: [(CGFloat, CGFloat)] = [
    (270, 112), (344, 112), (344, 96), (374, 120),
    (344, 144), (344, 128), (270, 128),
]

for scale in [1, 2] {
    guard let context = CGContext(
        data: nil, width: 640 * scale, height: 240 * scale,
        bitsPerComponent: 8, bytesPerRow: 640 * scale * 4,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        fatalError("Cannot create DMG background context")
    }
    context.scaleBy(x: CGFloat(scale), y: CGFloat(scale))
    context.setFillColor(CGColor(gray: 1, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: 640, height: 240))
    context.beginPath()
    for (index, point) in points.enumerated() {
        let position = CGPoint(x: point.0, y: point.1)
        if index == 0 {
            context.move(to: position)
        } else {
            context.addLine(to: position)
        }
    }
    context.closePath()
    context.setFillColor(CGColor(gray: 0, alpha: 1))
    context.fillPath()
    let suffix = scale == 1 ? ".png" : "@2x.png"
    let url = URL(fileURLWithPath: prefix + suffix)
    guard let image = context.makeImage(),
        let destination = CGImageDestinationCreateWithURL(
            url as CFURL, "public.png" as CFString, 1, nil
        )
    else {
        fatalError("Cannot create DMG background image")
    }
    let properties = [
        kCGImagePropertyDPIWidth as String: 72 * scale,
        kCGImagePropertyDPIHeight as String: 72 * scale,
    ]
    CGImageDestinationAddImage(destination, image, properties as CFDictionary)
    guard CGImageDestinationFinalize(destination) else {
        fatalError("Cannot write DMG background image")
    }
}
