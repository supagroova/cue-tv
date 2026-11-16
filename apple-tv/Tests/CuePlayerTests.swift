import Foundation
import Testing
@testable import SittingRoomTV

@Test func internalPlayerAcceptsOnlyItsSelectedDebridSource() throws {
    func media(_ address: String) throws -> CueMedia {
        let value: [String: Any] = ["playback": String(repeating: "a", count: 32), "url": address, "type": "series", "id": "tt1234", "video": "tt1234:1:1", "title": "Fixture", "resume": 30000]
        return try JSONDecoder().decode(CueMedia.self, from: JSONSerialization.data(withJSONObject: value))
    }
    #expect(try media("https://torrentio.strem.fun/realdebrid/fixture").validatedURL != nil)
    #expect(try media("https://torrentio.strem.fun/resolve/realdebrid/fixture").validatedURL != nil)
    for address in ["https://example.org/video.mkv", "http://torrentio.strem.fun/realdebrid/fixture", "https://torrentio.strem.fun.evil.example/realdebrid/fixture", "https://torrentio.strem.fun:8443/realdebrid/fixture", "https://user:secret@torrentio.strem.fun/realdebrid/fixture", "magnet:fixture"] {
        #expect(try media(address).validatedURL == nil)
    }
}
@Test func futureEpisodesWithFractionalReleaseDatesCannotStart() throws {
    let value = Data(#"{"id":"tt1234:2:1","season":2,"episode":1,"title":"Future","released":"2999-01-01T00:00:00.000Z"}"#.utf8)
    let episode = try JSONDecoder().decode(CueEpisode.self, from: value)
    #expect(!episode.available)
}
