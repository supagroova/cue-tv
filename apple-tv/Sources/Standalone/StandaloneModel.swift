import Foundation
import Observation

@MainActor @Observable
final class StandaloneModel {
    private(set) var snapshot: CoreSnapshot?
    private(set) var error: String?
    private var service: (any CoreService)?
    private var active: Task<Void, Never>?
    private var retirement: Task<Void, any Error>?
    private var generation = UUID()
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
        let token = UUID()
        generation = token
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

    func prepare(index: Int, expectedRevision: UInt64? = nil) async -> CoreTorrentRequest? {
        guard let service, let revision = snapshot?.model?.revision else { return nil }
        guard expectedRevision == nil || expectedRevision == revision else {
            error = "That source has changed. Please select it again."
            return nil
        }
        let token = generation
        do {
            let request = try await service.prepareTorrent(index: index, expectedRevision: revision)
            guard generation == token, !Task.isCancelled else { return nil }
            return request
        } catch {
            guard generation == token, !Task.isCancelled else { return nil }
            self.error = "This source could not be prepared. Select it again or choose another source."
            return nil
        }
    }
}
