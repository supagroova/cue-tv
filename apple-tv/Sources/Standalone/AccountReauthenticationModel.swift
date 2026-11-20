import Foundation
import Observation

/// Borrows the selected account; never creates, selects or deletes a namespace.
@MainActor @Observable
final class AccountReauthenticationModel: Identifiable {
    let id = UUID()
    private(set) var busy = false
    private(set) var error: String?
    private let home: StandaloneModel
    private let session: UInt64
    private var sequence: UInt64?
    private var operation: Task<Void, Never>?
    private var closing = false
    private var completed = false
    var canSubmit: Bool { operation == nil && sequence == nil && !closing && !completed }

    init(home: StandaloneModel, session: UInt64) { self.home = home; self.session = session }

    func submit(email: String, password: String, onSuccess: @escaping @MainActor () -> Void) {
        guard canSubmit else { return }
        busy = true
        error = nil
        operation = Task {
            defer { operation = nil; busy = false }
            do {
                let admitted = try await home.beginAccountRefresh(email: email, password: password, session: session)
                sequence = admitted
                let deadline = ContinuousClock.now.advanced(by: .seconds(45))
                while true {
                    try Task.checkCancellation()
                    guard home.snapshot?.session == session, home.snapshot?.status != .stopped else {
                        throw NativeCoreService.Failure.stopped
                    }
                    if let sync = home.snapshot?.model?.sync, sync.reauthSequence == admitted,
                       sync.reauthenticating != true {
                        if sync.state == .reauthenticationRequired {
                            throw NativeCoreService.Failure.requestFailed(home.snapshot?.model?.error ?? "reauthentication_failed")
                        }
                        completed = true
                        sequence = nil
                        onSuccess()
                        return
                    }
                    guard ContinuousClock.now < deadline else { throw NativeCoreService.Failure.requestTimeout }
                    try await Task.sleep(for: .milliseconds(50))
                }
            } catch {
                let failure = error
                if case NativeCoreService.Failure.refreshCleanupRequired(let pending) = failure {
                    sequence = pending
                }
                do {
                    try await cleanup()
                    if !Task.isCancelled { self.error = Self.message(for: failure) }
                } catch { self.error = "Sign-in could not stop. Please retry Cancel." }
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
        guard !completed else { return true }
        do { try await cleanup(); return true }
        catch { self.error = "Sign-in could not stop. Please retry Cancel."; return false }
    }

    private func cleanup() async throws {
        guard let sequence else { return }
        // Cleanup must run even when the submitting task was cancelled.
        let cancellation = Task { try await home.cancelAccountRefresh(sequence: sequence, session: session) }
        try await cancellation.value
        self.sequence = nil
    }

    private static func message(for error: any Error) -> String {
        switch error {
        case NativeCoreService.Failure.requestFailed("reauthentication_identity_mismatch"):
            "Use the same Stremio account. Your pending changes have been kept."
        case NativeCoreService.Failure.requestFailed("reauthentication_save_failed"):
            "Sign-in could not be saved. Your pending changes have been kept."
        case NativeCoreService.Failure.requestFailed("reauthentication_rejected"), NativeCoreService.Failure.invalidRequest:
            "Check your email and password, then try again."
        default:
            "Sign-in could not finish. Check your connection and try again."
        }
    }
}
