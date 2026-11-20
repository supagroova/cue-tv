import Foundation

/// Retains failed saves independently of the player view's lifetime.
@MainActor
final class PlaybackAccountingSession {
    private var current: (token: UInt64, owner: PlaybackAccountingOwner)?
    private var changing = false

    func acquire(token: UInt64, execute: @escaping @Sendable (CorePlaybackAction, UInt64, UInt64) async throws -> CorePlaybackProgress?) async -> PlaybackAccountingOwner? {
        guard token > 0, !changing else { return nil }
        if let current, current.token == token { return current.owner }
        changing = true
        defer { changing = false }
        guard await retireCurrent() else { return nil }
        let owner = PlaybackAccountingOwner(execute: execute)
        current = (token, owner)
        return owner
    }

    func close() async -> Bool {
        guard !changing else { return false }
        changing = true
        defer { changing = false }
        return await retireCurrent()
    }

    private func retireCurrent() async -> Bool {
        guard let current else { return true }
        guard await current.owner.close() else { return false }
        self.current = nil
        return true
    }
}
