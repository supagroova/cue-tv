import Foundation
import VLCKit

/// LibVLC messages may contain source URLs, paths and track titles. Never forward them.
nonisolated final class DiscardPlaybackLogger: NSObject, VLCLogging {
    var level: VLCLogLevel {
        get { VLCLogLevel(rawValue: 3)! }
        set { }
    }
    func handleMessage(_ message: String, logLevel: VLCLogLevel, context: VLCLogContext?) { }
}

@MainActor enum PlaybackLogging {
    static func makePlayer() -> VLCMediaPlayer {
        let library = VLCLibrary.shared()
        // An empty logger list unsets the libVLC callback; use an explicit sink.
        library.loggers = [DiscardPlaybackLogger()]
        return VLCMediaPlayer(library: library)
    }
}
