import Foundation
import Darwin

actor TVConnection {
    enum WakeResult: Sendable { case awake, readyToPair }
    private struct PowerReply: Decodable { var ok: Bool; var power: String? }
    private let session: URLSession

    init() {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 2
        configuration.timeoutIntervalForResource = 4
        configuration.waitsForConnectivity = false
        session = URLSession(configuration: configuration)
    }

    func isAvailable(_ target: TVTarget) async -> Bool {
        guard let base = target.baseURL else { return false }
        do {
            let (data, response) = try await session.data(from: base.appendingPathComponent("api/health"))
            guard (response as? HTTPURLResponse)?.statusCode == 200 else { return false }
            let reply = try JSONDecoder().decode(PowerReply.self, from: data)
            return reply.ok
        } catch { return false }
    }

    func wake(_ target: TVTarget, token: String?) async throws -> WakeResult {
        guard let base = target.baseURL else { throw TVRemoteError.invalidTarget }
        let packet = try WakePacket.make(macAddress: target.macAddress)
        let deadline = ContinuousClock.now.advanced(by: .seconds(45))
        for attempt in 0..<18 {
            try Task.checkCancellation()
            guard ContinuousClock.now < deadline else { break }
            if attempt % 3 == 0 { try? send(packet, to: target.host) }
            if let token {
                var request = URLRequest(url: base.appendingPathComponent("api/power"))
                request.httpMethod = "POST"; request.httpBody = Data("{\"state\":\"on\"}".utf8)
                request.setValue("application/json", forHTTPHeaderField: "Content-Type")
                request.setValue("Bearer " + token, forHTTPHeaderField: "Authorization")
                do {
                    let (data, response) = try await session.data(for: request)
                    if (response as? HTTPURLResponse)?.statusCode == 401 { return .readyToPair }
                    if let reply = try? JSONDecoder().decode(PowerReply.self, from: data), reply.ok, reply.power == "on" { return .awake }
                } catch is CancellationError { throw CancellationError() }
                catch { /* The TV is still waking. The next attempt is bounded by the same timeout. */ }
            } else if await isAvailable(target) { return .readyToPair }
            try await Task.sleep(for: .seconds(1))
        }
        throw TVRemoteError.notReachable
    }

    // A directed magic packet needs Local Network permission but no multicast entitlement.
    // Broadcast wake must not be enabled without Apple's separate multicast entitlement.
    private func send(_ packet: Data, to host: String) throws {
        let fd = Darwin.socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP)
        guard fd >= 0 else { throw TVRemoteError.notReachable }
        defer { Darwin.close(fd) }
        var address = sockaddr_in()
        address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
        address.sin_family = sa_family_t(AF_INET); address.sin_port = UInt16(9).bigEndian
        guard host.withCString({ inet_pton(AF_INET, $0, &address.sin_addr) }) == 1 else { throw TVRemoteError.invalidTarget }
        let count = packet.withUnsafeBytes { bytes in
            withUnsafePointer(to: &address) { pointer in
                pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    Darwin.sendto(fd, bytes.baseAddress, bytes.count, 0, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
                }
            }
        }
        guard count == packet.count else {
            if errno == EPERM || errno == EACCES { throw TVRemoteError.localNetworkDenied }
            throw TVRemoteError.notReachable
        }
    }
}
