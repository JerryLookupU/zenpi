import Foundation
import CoreGraphics
import Vision

// The harness supplies an uncompressed PGM reconstructed from real PTY cells.
let bytes = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
let separators = bytes.indices.filter { bytes[$0] == 10 }.prefix(3)
guard separators.count == 3 else { fatalError("Invalid PGM") }
let headerEnd = separators.last! + 1
let header = String(data: bytes.prefix(headerEnd), encoding: .ascii)!.split(whereSeparator: { $0.isWhitespace })
guard header.count == 4, header[0] == "P5", header[3] == "255",
      let width = Int(header[1]), let height = Int(header[2]), bytes.count - headerEnd == width * height
else { fatalError("Invalid PGM dimensions") }
let pixels = bytes.dropFirst(headerEnd) as NSData
let provider = CGDataProvider(data: pixels)!
let image = CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 8,
    bytesPerRow: width, space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGBitmapInfo(rawValue: 0),
    provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)!
let request = VNDetectBarcodesRequest()
request.symbologies = [.qr]
try VNImageRequestHandler(cgImage: image).perform([request])
guard let results = request.results, results.count == 1, let payload = results[0].payloadStringValue
else { fatalError("Exactly one decodable terminal QR required") }
print(payload)
