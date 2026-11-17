import Foundation

/// Private selected-source data; never include URLs in presentation diagnostics.
nonisolated enum CorePlaybackRequest: Sendable {
    case torrent(CoreTorrentRequest)
    case https(CoreHTTPSRequest)
}
