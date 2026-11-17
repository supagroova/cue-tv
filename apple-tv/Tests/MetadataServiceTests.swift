import Foundation
import Testing
@testable import SittingRoomTV

@Suite struct MetadataServiceTests {
    @Test func invalidSourceReportsFailureAndConcurrentStopsReleaseHandle() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let service = NativeMetadataService()
        try await service.start(magnet: "invalid", output: directory, discoverPeers: false)
        var snapshot = try await service.snapshot()
        for _ in 0..<100 where snapshot.state == .loading {
            try await Task.sleep(for: .milliseconds(10))
            snapshot = try await service.snapshot()
        }
        #expect(snapshot.state == .failed)
        #expect(snapshot.errorCode == "invalid_metadata")
        async let first: Void = service.stop()
        async let second: Void = service.stop()
        try await first
        try await second
        try await service.start(magnet: "invalid", output: directory, discoverPeers: false)
        try await service.stop()
        #expect(try FileManager.default.contentsOfDirectory(atPath: directory.path).isEmpty)
    }
}
