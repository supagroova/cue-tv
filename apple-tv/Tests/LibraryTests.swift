import Foundation
import Testing
@testable import SittingRoomTV

@Test func stremioLinksValidateTypesAndIdentifiers() throws {
    func title(_ id: String, _ type: String) throws -> Title {
        let data = try JSONSerialization.data(withJSONObject: ["id": id, "type": type, "title": "Fixture"])
        return try JSONDecoder().decode(Title.self, from: data)
    }
    #expect(try title("tt1234", "series").stremioURL?.absoluteString == "stremio:///detail/series/tt1234")
    #expect(try title("tt1234", "movie").stremioURL?.absoluteString == "stremio:///detail/movie/tt1234/tt1234")
    #expect(try title("tt1234?redirect=evil", "series").stremioURL == nil)
    #expect(try title("tt1234", "channel").stremioURL == nil)
}
@Test func connectionAddressStaysOnHomeNetwork() {
    #expect(TVAddress.normalise("192.168.50.40")?.absoluteString == "http://192.168.50.40:5400")
    #expect(TVAddress.normalise("http://192.168.50.40:5400/somewhere?key=remove")?.absoluteString == "http://192.168.50.40:5400")
    #expect(TVAddress.normalise("https://example.org") == nil)
    #expect(TVAddress.normalise("http://secret@192.168.50.40") == nil)
    #expect(TVAddress.normalise("file:///private/local") == nil)
    #expect(TVAddress.normalise("999.168.50.40") == nil)
}
@Test func optionalArtworkAndUnconfiguredTrendingDecode() throws {
    let data = Data(#"{"series":[{"id":"tt1","type":"series","title":"A show"}],"movies":[]}"#.utf8)
    let catalogue = try JSONDecoder().decode(Catalogue.self, from: data)
    #expect(catalogue.trendingSeries == nil)
    #expect(catalogue.series.first?.posterURL == nil)
    #expect(Title.imageURL("http://example.org/image.jpg") == nil)
}

@Test func phoneHandoffRoutesOnlyRecognisedServicesAndValidVideoIDs() throws {
    func request(_ json: String) throws -> TVEvent.TitleRequest {
        try JSONDecoder().decode(TVEvent.TitleRequest.self, from: Data(json.utf8))
    }
    #expect(try request(#"{"serial":1,"type":"series","id":"tt1234"}"#).url?.scheme == "stremio")
    #expect(try request(#"{"serial":2,"service":"youtube","type":"video","id":"abc_DEF-123"}"#).url?.absoluteString == "youtube://www.youtube.com/watch?v=abc_DEF-123")
    #expect(try request(#"{"serial":3,"service":"youtube","type":"video","id":"abc_DEF-123&x=1"}"#).url == nil)
    #expect(try request(#"{"serial":4,"service":"unknown","type":"series","id":"tt1234"}"#).url == nil)
    #expect(try request(#"{"serial":5,"service":"youtube","type":"series","id":"abc_DEF-123"}"#).url == nil)
}

@Test func selectedStremioPlayerLinksStayInsideThePlayerScheme() {
    #expect(StremioPlayerLink(uri: "stremio:///player/encoded/provider/meta/series/tt1/tt1%3A1%3A2").validatedURL != nil)
    for raw in ["https://example.org/player/foo", "stremio://evil/player/foo", "stremio:///detail/series/tt1", "stremio:///player/foo?redirect=evil", "stremio:///player/foo#fragment"] {
        #expect(StremioPlayerLink(uri: raw).validatedURL == nil)
    }
}
