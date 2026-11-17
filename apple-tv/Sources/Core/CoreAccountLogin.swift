import Foundation

/// Single-use owner of a candidate account until verified selection is handed to the UI.
actor CoreAccountLogin {
    enum Failure: Error, Equatable { case busy, authenticationFailed, timeout, invalidIdentity }
    nonisolated let namespace: UUID
    private let storage: CoreStorage
    private let selection: CoreAccountSelection
    private var service: NativeCoreService?
    private var operation: Task<NativeCoreService, any Error>?
    private var cancelled = false
    private var used = false
    private var handedOff = false
    private var preparationStarted = false

    init(root: URL? = nil, selection: CoreAccountSelection = .shared) {
        let id = UUID()
        namespace = id
        storage = CoreStorage.account(id: id, root: root)
        self.selection = selection
    }

    func signIn(email: String, password: String) async throws -> NativeCoreService {
        try Task.checkCancellation()
        guard !cancelled else { throw CancellationError() }
        guard !used else { throw Failure.busy }
        used = true
        let task = Task { try await self.authenticate(email: email, password: password) }
        operation = task
        do {
            let result = try await withTaskCancellationHandler {
                try await task.value
            } onCancel: { task.cancel() }
            try Task.checkCancellation()
            guard !task.isCancelled else { throw CancellationError() }
            operation = nil
            handedOff = true
            return result
        } catch {
            operation = nil
            try await discardCandidate()
            throw error
        }
    }

    private func authenticate(email: String, password: String) async throws -> NativeCoreService {
        try Task.checkCancellation()
        guard try await selection.load() == nil else { throw Failure.busy }
        try Task.checkCancellation()
        preparationStarted = true
        let configuration = try await storage.prepare()
        let candidate = NativeCoreService(persistence: configuration, startup: .newAccount)
        service = candidate
        try Task.checkCancellation()
        try await candidate.start()
        try await wait(candidate, authenticated: false)
        try await candidate.login(email: email, password: password)
        try await wait(candidate, authenticated: true)
        guard let userID = try await candidate.accountIdentity() else { throw Failure.invalidIdentity }
        try Task.checkCancellation()
        try await selection.selectNew(.init(namespace: namespace, userID: userID))
        try Task.checkCancellation()
        return candidate
    }

    private func wait(_ candidate: NativeCoreService, authenticated: Bool) async throws {
        let deadline = ContinuousClock.now + .seconds(45)
        while true {
            try Task.checkCancellation()
            let state = try await candidate.snapshot()
            guard state.status != .stopped, state.error == nil else {
                throw Failure.authenticationFailed
            }
            if state.status == .ready, !authenticated ||
                (state.model?.authenticated == true && state.model?.pendingEffects == 0) { return }
            // Ancillary add-on/library fetch failures do not undo successful authentication.
            if state.model?.error != nil && state.model?.authenticated != true { throw Failure.authenticationFailed }
            guard ContinuousClock.now < deadline else { throw Failure.timeout }
            try await Task.sleep(for: .milliseconds(50))
        }
    }

    /// Explicit cleanup retry remains available if a prior shutdown or Keychain operation failed.
    func cancel() async throws {
        guard !handedOff else { throw Failure.busy }
        cancelled = true
        if let operation {
            operation.cancel()
            _ = try? await operation.value
        }
        try await discardCandidate()
    }

    private func discardCandidate() async throws {
        // Keychain failure must not leave candidate network work running.
        if let service { try await service.stop() }
        try await selection.clear(ifMatching: namespace)
        if let service {
            try await service.discardAccountStorage(storage)
            self.service = nil
        } else if preparationStarted {
            // This instance allocated a new UUID and never started a native owner.
            try await storage.removeUnopenedAccount()
        }
        preparationStarted = false
    }
}
