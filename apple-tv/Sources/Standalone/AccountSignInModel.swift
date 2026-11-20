import Foundation
import Observation

@MainActor @Observable
final class AccountSignInModel {
    private(set) var busy = false
    private(set) var error: String?
    private let root: URL?
    private let selection: CoreAccountSelection
    private let transport: AccountLoginTransport
    private var attempt: CoreAccountLogin?
    private var candidate: NativeCoreService?
    private var operation: Task<Void, Never>?
    private var closing = false
    private var adopted = false
    init(root: URL? = nil, selection: CoreAccountSelection = .shared, transport: AccountLoginTransport = .stremio) {
        self.root = root; self.selection = selection; self.transport = transport
    }

    func submit(email: String, password: String, home: StandaloneModel, onSuccess: @escaping @MainActor () -> Void) {
        guard operation == nil, !closing, !adopted else { return }
        busy = true
        error = nil
        operation = Task {
            defer { operation = nil; busy = false }
            do {
                try await cleanup()
                try Task.checkCancellation()
                let next = CoreAccountLogin(root: root, selection: selection, transport: transport)
                attempt = next
                let service = try await next.signIn(email: email, password: password)
                candidate = service
                try Task.checkCancellation()
                try await home.replace(with: service)
                // Successful adoption is the ownership transfer point.
                adopted = true
                candidate = nil
                attempt = nil
                onSuccess()
            } catch {
                do {
                    try await cleanup()
                    if !Task.isCancelled { self.error = "Sign-in failed. Check your details and connection, then try again." }
                } catch {
                    self.error = "The sign-in session could not close. Please retry Cancel."
                }
            }
        }
    }

    @discardableResult
    func cancel() async -> Bool {
        guard !closing else { return false }
        closing = true
        defer { closing = false }
        let pending = operation
        pending?.cancel()
        await pending?.value
        guard !adopted else { return true }
        do { try await cleanup(); return true }
        catch { self.error = "The sign-in session could not close. Please retry Cancel."; return false }
    }

    private func cleanup() async throws {
        if let candidate, let attempt {
            try await candidate.stop()
            try await selection.clear(ifMatching: attempt.namespace)
            try await candidate.discardAccountStorage(CoreStorage.account(id: attempt.namespace, root: root))
            self.candidate = nil
            self.attempt = nil
        } else if let attempt {
            try await attempt.cancel()
            self.attempt = nil
        }
    }
}
