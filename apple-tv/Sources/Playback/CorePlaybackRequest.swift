import Foundation

/// Private selected-source data; never include URLs in presentation diagnostics.
nonisolated enum CorePlaybackRequest: Sendable {
    case torrent(CoreTorrentRequest)
    case https(CoreHTTPSRequest)
    var subtitles: [CoreSubtitle] {
        switch self { case .torrent(let source): source.subtitles; case .https(let source): source.subtitles }
    }
}
