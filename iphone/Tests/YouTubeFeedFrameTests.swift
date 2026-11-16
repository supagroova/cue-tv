import Foundation
import Testing
@testable import CouchRemote

struct YouTubeFeedFrameTests {
    private func payload() -> [String: Any] {
        ["visible": true, "frame": ["x": 18.0, "y": 340.0, "width": 357.0, "height": 560.0],
         "clip": ["x": 18.0, "y": 340.0, "width": 357.0, "height": 420.0],
         "viewport": ["width": 393.0, "height": 852.0]]
    }
    @Test func visibleFeedPreservesItsFullFrameAndClippedRegion() throws {
        let frame = try #require(YouTubeFeedFrame(payload: payload()))
        #expect(frame.full.height == 560); #expect(frame.clip.height == 420)
        #expect(frame.viewport.width == 393)
        var scrolled = payload(); scrolled["frame"] = ["x": 18.0, "y": -100.0, "width": 357.0, "height": 560.0]
        scrolled["clip"] = ["x": 18.0, "y": 0.0, "width": 357.0, "height": 460.0]
        #expect(YouTubeFeedFrame(payload: scrolled) != nil)
    }
    @Test func hiddenMalformedAndOutOfBoundsFramesAreRejected() {
        #expect(YouTubeFeedFrame(payload: ["visible": false]) == nil)
        #expect(YouTubeFeedFrame(payload: [:]) == nil)
        for rect in [
            ["x": Double.nan, "y": 340.0, "width": 357.0, "height": 560.0],
            ["x": 18.0, "y": 340.0, "width": -1.0, "height": 560.0],
            ["x": 18.0, "y": 340.0, "width": 357.0, "height": 9000.0],
        ] {
            var value = payload(); value["frame"] = rect
            #expect(YouTubeFeedFrame(payload: value) == nil)
        }
        var outside = payload(); outside["clip"] = ["x": 18.0, "y": 340.0, "width": 600.0, "height": 420.0]
        #expect(YouTubeFeedFrame(payload: outside) == nil)
    }
}
