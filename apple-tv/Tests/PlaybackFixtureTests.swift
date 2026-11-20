import Foundation
import Testing
@testable import SittingRoomTV

@Test func fixtureRejectsPathsOutsideBundle() throws {
    let json = #"{"id":"sample","title":"Sample","filename":"../private.mp4","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","durationSeconds":12}"#.data(using: .utf8)!
    #expect(throws: (any Error).self) { try JSONDecoder().decode(PlaybackFixture.self, from: json) }
}

@Test func bundledFixtureManifestIsUsable() throws {
    let fixtures = try PlaybackFixture.load(from: Bundle.main)
    #expect(fixtures.count == 3)
    #expect(Set(fixtures.map(\.id)).count == 3)
    for fixture in fixtures {
        #expect(fixture.durationSeconds == 12)
        #expect(fixture.sha256.count == 64)
        #expect(fixture.url(in: Bundle.main) != nil)
    }
}
