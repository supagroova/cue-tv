import Foundation
import CueCore

nonisolated struct CoreSnapshot: Decodable, Sendable {
    enum Status: String, Decodable, Sendable { case starting, ready, stopped }
    struct Model: Decodable, Sendable {
        let revision: UInt64
        let stopped: Bool
        let interfaceLanguage: String
        let addonCount: Int
        let pendingEffects: Int
        let error: String?
    }
    let version: Int
    let session: UInt64
    let status: Status
    let model: Model?
    let error: String?
}

/// Serializes native admission off the UI actor. No account data crosses this initial service.
actor NativeCoreService {
    enum Failure: Error, Equatable { case unavailable, stopped, invalidRequest, busy, malformedSnapshot, shutdownTimeout }
    private var handle: UInt64?
    private var retirement: Task<Void, any Error>?

    func start() throws {
        guard retirement == nil else { throw Failure.busy }
        guard handle == nil else { return }
        guard cue_core_abi_version() == 1 else { throw Failure.unavailable }
        let created = cue_core_create()
        guard created != 0 else { throw Failure.unavailable }
        handle = created
    }

    func snapshot() throws -> CoreSnapshot {
        guard let handle, retirement == nil else { throw Failure.stopped }
        var bytes = [UInt8](repeating: 0, count: 16 * 1024)
        let length = bytes.withUnsafeMutableBufferPointer { cue_core_snapshot(handle, $0.baseAddress, $0.count) }
        guard length > 0, length <= bytes.count else { throw Failure.malformedSnapshot }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        guard let value = try? decoder.decode(CoreSnapshot.self, from: Data(bytes.prefix(length))),
              value.version == 1, value.session == handle else { throw Failure.malformedSnapshot }
        return value
    }

    func setInterfaceLanguage(_ language: String) throws {
        guard let handle, retirement == nil else { throw Failure.stopped }
        let bytes = try JSONSerialization.data(withJSONObject: ["version": 1, "action": "set_language", "language": language])
        let result = bytes.withUnsafeBytes { cue_core_dispatch(handle, $0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        switch result {
        case 0: return
        case -2: throw Failure.invalidRequest
        case -3: throw Failure.busy
        default: throw Failure.stopped
        }
    }

    func stop() async throws {
        guard let handle else { return }
        let task: Task<Void, any Error>
        if let retirement { task = retirement }
        else {
            task = Task.detached { try await Self.retire(handle) }
            retirement = task
        }
        do {
            try await task.value
            // Actor reentrancy: a second stop may resume after a new start.
            if self.handle == handle { self.handle = nil; retirement = nil }
        } catch {
            if self.handle == handle { retirement = nil }
            throw error
        }
    }

    private nonisolated static func retire(_ handle: UInt64) async throws {
        _ = cue_core_stop(handle)
        let deadline = ContinuousClock.now.advanced(by: .seconds(5))
        while cue_core_stop_status(handle) == 0 {
            guard ContinuousClock.now < deadline else { throw Failure.shutdownTimeout }
            try await Task.sleep(for: .milliseconds(10))
        }
        guard cue_core_release(handle) == 1 else { throw Failure.unavailable }
    }

    deinit {
        if let handle, retirement == nil {
            Task.detached { try? await Self.retire(handle) }
        }
    }
}
