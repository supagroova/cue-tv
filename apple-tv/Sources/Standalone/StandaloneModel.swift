import Foundation
import Observation

@MainActor @Observable
final class StandaloneModel {
    private(set) var snapshot: CoreSnapshot?
    private(set) var error: String?
    private var service: (any CoreService)?
    private let playbackSession = PlaybackAccountingSession()
    private var active: Task<Void, Never>?
    private var retirement: Task<Void, any Error>?
    private var generation = UUID()
    private(set) var retryingSync = false
    private(set) var signingOut = false
    var needsSignOutRetry: Bool { pendingSignOut != nil }
    private var pendingSignOut: (owner: NativeCoreService?, account: CoreAccountSelection.Account)?
    private let accountSelection: CoreAccountSelection
    private let accountRoot: URL?
    private var restorationAccount: CoreAccountSelection.Account?
    var canSignOut: Bool { snapshot?.model?.authenticated == true || restorationAccount != nil || pendingSignOut != nil }
    init(service: (any CoreService)? = nil, accountSelection: CoreAccountSelection = .shared, accountRoot: URL? = nil) {
        self.service = service
        self.accountSelection = accountSelection
        self.accountRoot = accountRoot
    }

    func run() async {
        guard !signingOut, pendingSignOut == nil else {
            error = "Account cleanup must finish before starting another session. Retry sign-out."
            return
        }
        let token = UUID()
        generation = token
        retryingSync = false
        let previous = active
        let pendingRetirement = retirement
        previous?.cancel()
        let task = Task { [weak self] in
            await previous?.value
            guard let self, !Task.isCancelled, self.generation == token else { return }
            do { try await pendingRetirement?.value }
            catch {
                if self.generation == token { self.error = "The previous session could not stop. Please retry." }
                return
            }
            guard !Task.isCancelled, self.generation == token else { return }
            await self.poll(generation: token)
        }
        active = task
        await withTaskCancellationHandler {
            await task.value
        } onCancel: { task.cancel() }
        if generation == token { active = nil }
    }

    /// Ownership transfers only on successful return; the caller retains a rejected candidate.
    func replace(with replacement: (any CoreService)?) async throws {
        guard !signingOut, pendingSignOut == nil else { throw NativeCoreService.Failure.stopped }
        try await replaceOwner(with: replacement)
    }

    /// Retains the retiring account across cleanup failures so a retry cannot lose its owner.
    func signOut(selection override: CoreAccountSelection? = nil, root: URL? = nil) async throws {
        let selection = override ?? accountSelection
        let root = root ?? accountRoot
        guard !signingOut else { throw NativeCoreService.Failure.stopped }
        signingOut = true
        defer { signingOut = false }
        if pendingSignOut == nil {
            guard let account = try await selection.load() else { throw NativeCoreService.Failure.stopped }
            let owner = service as? NativeCoreService
            // No owner is safe only when restoration failed before creating one.
            guard owner != nil || (service == nil && restorationAccount == account) else {
                throw NativeCoreService.Failure.stopped
            }
            pendingSignOut = (owner, account)
        }
        guard let pending = pendingSignOut else { throw NativeCoreService.Failure.stopped }
        try await replaceOwner(with: nil)
        try await pending.owner?.stop()
        try await selection.clear(ifMatching: pending.account.namespace)
        let storage = CoreStorage.account(id: pending.account.namespace, root: root)
        if let owner = pending.owner { try await owner.discardAccountStorage(storage) }
        else { try await storage.removeUnopenedAccount() }
        pendingSignOut = nil
        restorationAccount = nil
    }

    private func replaceOwner(with replacement: (any CoreService)?) async throws {
        guard await closePlayback() else { throw NativeCoreService.Failure.requestFailed("playback_save_failed") }
        let token = UUID()
        generation = token
        retryingSync = false
        snapshot = nil
        let previous = active
        let oldService = service
        let pendingRetirement = retirement
        previous?.cancel()
        let retiring = Task {
            _ = try? await pendingRetirement?.value
            await previous?.value
            if let oldService { try await oldService.stop() }
        }
        retirement = retiring
        try await retiring.value
        guard generation == token, !Task.isCancelled else { throw CancellationError() }
        retirement = nil
        active = nil
        service = replacement
        snapshot = nil
        error = nil
    }

    private func poll(generation token: UUID) async {
        error = nil
        snapshot = nil
        guard await closePlayback() else { return }
        do {
            if service == nil {
                let account = try await accountSelection.load()
                guard generation == token, !Task.isCancelled else { throw CancellationError() }
                restorationAccount = account
                let selected: NativeCoreService
                if let account {
                    let configuration = try await CoreStorage.account(id: account.namespace, root: accountRoot).prepareExistingAccount()
                    selected = NativeCoreService(persistence: configuration, startup: .restoreAccount(account.userID))
                } else { selected = try await NativeCoreService.guest() }
                guard generation == token, !Task.isCancelled else { throw CancellationError() }
                service = selected
            }
            guard let service else { return }
            try await service.start()
            while !Task.isCancelled {
                let next = try await service.snapshot()
                guard generation == token, !Task.isCancelled else { throw CancellationError() }
                snapshot = next
                guard snapshot?.status != .stopped else { throw NativeCoreService.Failure.stopped }
                try await Task.sleep(for: .milliseconds(100))
            }
        } catch is CancellationError { }
        catch { self.error = "The Stremio session could not start. Please try again." }
        guard await closePlayback() else { return }
        if let service { try? await service.stop() }
    }

    @discardableResult
    func perform(expectedRevision: UInt64? = nil, _ action: @Sendable (any CoreService, UInt64) async throws -> Void) async -> Bool {
        guard let service, let revision = snapshot?.model?.revision else { return false }
        guard expectedRevision == nil || expectedRevision == revision else {
            error = "That selection has changed. Please select it again."
            return false
        }
        let token = generation
        error = nil
        do {
            try await action(service, revision)
            return generation == token && !Task.isCancelled
        } catch {
            guard generation == token, !Task.isCancelled else { return false }
            self.error = "That selection is no longer available. Please try again."
            return false
        }
    }

    func beginAccountRefresh(email: String, password: String, session: UInt64) async throws -> UInt64 {
        guard let service, snapshot?.session == session, !signingOut, pendingSignOut == nil else {
            throw NativeCoreService.Failure.stopped
        }
        let token = generation
        let sequence = try await service.reauthenticate(email: email, password: password)
        guard generation == token, !Task.isCancelled else {
            let cancellation = Task { try await service.cancelReauthentication(sequence: sequence) }
            do { try await cancellation.value }
            catch { throw NativeCoreService.Failure.refreshCleanupRequired(sequence) }
            throw CancellationError()
        }
        return sequence
    }

    func cancelAccountRefresh(sequence: UInt64, session: UInt64) async throws {
        guard let service, snapshot?.session == session else { return }
        try await service.cancelReauthentication(sequence: sequence)
    }

    func retrySync() async {
        guard let service, !signingOut, pendingSignOut == nil, !retryingSync,
              snapshot?.model?.authenticated == true,
              snapshot?.model?.sync?.state == .waiting else { return }
        let token = generation
        retryingSync = true
        defer { if generation == token { retryingSync = false } }
        do { try await service.retrySync() }
        catch {
            guard generation == token, !Task.isCancelled else { return }
            self.error = "Sync could not restart. Your pending changes have not been discarded."
        }
    }

    /// The parent retains failed accounting even after the full-screen view disappears.
    func accounting(for request: CorePlaybackRequest) async -> PlaybackAccountingOwner? {
        guard let service, !signingOut, pendingSignOut == nil else { return nil }
        let token = generation
        let playbackToken = request.playbackToken
        let owner = await playbackSession.acquire(token: playbackToken) { action, time, duration in
            try await service.playback(token: playbackToken, action: action, time: time, duration: duration)
        }
        guard generation == token, !Task.isCancelled else { return nil }
        return owner
    }

    @discardableResult
    func closePlayback() async -> Bool {
        guard await playbackSession.close() else {
            error = "Playback progress could not be saved. Please retry before starting another source."
            return false
        }
        return true
    }

    func prepare(index: Int, expectedRevision: UInt64? = nil) async -> CoreTorrentRequest? {
        await prepareSource(index: index, expectedRevision: expectedRevision) { try await $0.prepareTorrent(index: index, expectedRevision: $1) }
    }
    func subtitleDiscovery(for request: CorePlaybackRequest) -> @MainActor () async throws -> CoreSubtitleDiscovery {
        let owner = service
        let token = generation
        let identity: (String, String, String)
        switch request {
        case .torrent(let source): identity = (source.metaId, source.videoId, source.kind)
        case .https(let source): identity = (source.metaId, source.videoId, source.kind)
        }
        return { [weak self] in
            guard let self, self.generation == token, let owner else { throw CancellationError() }
            let result = try await owner.discoverSubtitles(metaId: identity.0, videoId: identity.1, kind: identity.2)
            try Task.checkCancellation()
            guard self.generation == token else { throw CancellationError() }
            return result
        }
    }
    func prepareHTTPS(index: Int, expectedRevision: UInt64? = nil) async -> CoreHTTPSRequest? {
        await prepareSource(index: index, expectedRevision: expectedRevision) { try await $0.prepareHTTPS(index: index, expectedRevision: $1) }
    }
    private func prepareSource<Request: Sendable>(index: Int, expectedRevision: UInt64?, action: @Sendable (any CoreService, UInt64) async throws -> Request) async -> Request? {
        guard let service, let revision = snapshot?.model?.revision else { return nil }
        guard expectedRevision == nil || expectedRevision == revision else {
            error = "That source has changed. Please select it again."
            return nil
        }
        let token = generation
        guard await closePlayback() else { return nil }
        do {
            let current = try await service.snapshot()
            guard generation == token, !Task.isCancelled else { return nil }
            guard current.model?.revision == revision else {
                snapshot = current
                error = "That source has changed. Please select it again."
                return nil
            }
            let request = try await action(service, revision)
            guard generation == token, !Task.isCancelled else { return nil }
            return request
        } catch {
            guard generation == token, !Task.isCancelled else { return nil }
            self.error = "This source could not be prepared. Select it again or choose another source."
            return nil
        }
    }
}
