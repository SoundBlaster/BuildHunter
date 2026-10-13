import Foundation
import CoreGraphics
import ImageIO

// Strip only fully opaque alpha, at unchanged pixel dimensions and in the source color space.
struct Preparation: Decodable { let source: String; let output: String; let pixels: [Int] }
enum PreparationError: Error { case invalidImage, transparentPixels, encoding, changedPixels, unsupportedOrientation, wrongDimensions }

func pixels(_ image: CGImage, space: CGColorSpace, depth: Int) throws -> Data {
    let stride = 4 * depth / 8
    var bytes = Data(count: image.width * image.height * stride)
    try bytes.withUnsafeMutableBytes { buffer in
        guard let context = CGContext(data: buffer.baseAddress, width: image.width, height: image.height,
            bitsPerComponent: depth, bytesPerRow: image.width * stride, space: space,
            bitmapInfo: (depth == 8 ? CGBitmapInfo.byteOrder32Big : CGBitmapInfo.byteOrder16Big).rawValue
                | CGImageAlphaInfo.premultipliedLast.rawValue) else {
            throw PreparationError.invalidImage
        }
        context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
    }
    return bytes
}

// Lossless pixel permutation. XCTest PNGs can store landscape as portrait pixels + EXIF orientation.
func oriented(_ bytes: Data, width: Int, height: Int, orientation: Int, stride: Int) throws -> (Data, Int, Int) {
    guard [1, 3, 6, 8].contains(orientation) else { throw PreparationError.unsupportedOrientation }
    if orientation == 1 { return (bytes, width, height) }
    let rotated = orientation == 6 || orientation == 8
    let outputWidth = rotated ? height : width
    let outputHeight = rotated ? width : height
    var result = Data(count: bytes.count)
    result.withUnsafeMutableBytes { output in
        bytes.withUnsafeBytes { input in
            for y in 0..<height {
                for x in 0..<width {
                    let target: (Int, Int)
                    switch orientation {
                    case 3: target = (width - 1 - x, height - 1 - y)
                    case 6: target = (height - 1 - y, x)
                    default: target = (y, width - 1 - x)
                    }
                    let from = (y * width + x) * stride
                    let to = (target.1 * outputWidth + target.0) * stride
                    output.baseAddress!.advanced(by: to).copyMemory(from: input.baseAddress!.advanced(by: from), byteCount: stride)
                }
            }
        }
    }
    return (result, outputWidth, outputHeight)
}

do {
    let preparations = try JSONDecoder().decode([Preparation].self, from: Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1])))
    for item in preparations {
        let source = URL(fileURLWithPath: item.source)
        let output = URL(fileURLWithPath: item.output)
        guard let reader = CGImageSourceCreateWithURL(source as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(reader, 0, nil), [8, 16].contains(image.bitsPerComponent),
              let space = image.colorSpace, space.model == .rgb else {
            throw PreparationError.invalidImage
        }
        let properties = CGImageSourceCopyPropertiesAtIndex(reader, 0, nil) as? [CFString: Any]
        let orientation = properties?[kCGImagePropertyOrientation] as? Int ?? 1
        let depth = image.bitsPerComponent
        let componentBytes = depth / 8
        let stride = 4 * componentBytes
        let (rgba, width, height) = try oriented(pixels(image, space: space, depth: depth), width: image.width,
                                               height: image.height, orientation: orientation, stride: stride)
        guard [width, height] == item.pixels else { throw PreparationError.wrongDimensions }
        var rgb = Data(count: width * height * 3 * componentBytes)
        try rgb.withUnsafeMutableBytes { destination in
            try rgba.withUnsafeBytes { source in
                for pixel in 0..<(width * height) {
                    let from = pixel * stride
                    for alpha in (from + 3 * componentBytes)..<(from + stride) {
                        guard source[alpha] == 255 else { throw PreparationError.transparentPixels }
                    }
                    destination.baseAddress!.advanced(by: pixel * 3 * componentBytes)
                        .copyMemory(from: source.baseAddress!.advanced(by: from), byteCount: 3 * componentBytes)
                }
            }
        }
        guard let provider = CGDataProvider(data: rgb as CFData),
              let opaque = CGImage(width: width, height: height, bitsPerComponent: depth, bitsPerPixel: 3 * depth,
                bytesPerRow: width * 3 * componentBytes, space: space,
                bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue | (depth == 16 ? CGBitmapInfo.byteOrder16Big.rawValue : 0)),
                provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent) else {
            throw PreparationError.invalidImage
        }
        try FileManager.default.createDirectory(at: output.deletingLastPathComponent(), withIntermediateDirectories: true)
        guard let writer = CGImageDestinationCreateWithURL(output as CFURL, "public.png" as CFString, 1, nil) else {
            throw PreparationError.encoding
        }
        CGImageDestinationAddImage(writer, opaque, nil)
        guard CGImageDestinationFinalize(writer), let result = CGImageSourceCreateWithURL(output as CFURL, nil),
              let decoded = CGImageSourceCreateImageAtIndex(result, 0, nil) else { throw PreparationError.encoding }
        guard decoded.width == width, decoded.height == height,
              decoded.bitsPerComponent == depth,
              try pixels(decoded, space: space, depth: depth) == rgba else { throw PreparationError.changedPixels }
    }
    print("Prepared \(preparations.count) native opaque PNGs; decoded pixel equality verified")
} catch {
    fputs("PNG preparation failed: \(error)\n", stderr)
    exit(1)
}
