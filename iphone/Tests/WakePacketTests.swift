import Testing
import Foundation
@testable import CouchRemote

struct WakePacketTests {
    @Test @MainActor func bundledTargetIsReadyToUse() {
        #expect(TVTarget.initial().isValid)
    }
    @Test func packetContainsSyncBytesAndSixteenCompleteMACCopies() throws {
        let mac: [UInt8] = [0x02,0x00,0x00,0x00,0x00,0x01]
        let packet = try WakePacket.make(macAddress: "02-00-00-00-00-01")
        #expect(packet.count == 102)
        #expect(Array(packet.prefix(6)) == Array(repeating: UInt8(255), count: 6))
        for index in 0..<16 { #expect(Array(packet[(6 + index*6)..<(12 + index*6)]) == mac) }
    }
    @Test func invalidAndBroadcastTargetsAreRejected() {
        for mac in ["", "ff:ff:ff:ff:ff:ff", "00:00:00:00:00:00", "01:00:00:00:00:01", "02:00:00:00:00", "02:00:00:00:00:zz", "020000000001"] {
            #expect(throws: TVRemoteError.self) { try WakePacket.make(macAddress: mac) }
        }
    }
    @Test func onlyExplicitPrivateIPv4TargetsAreAccepted() {
        for ip in ["192.168.50.40","10.0.0.2","172.16.1.2","172.31.255.254"] { #expect(TVTarget.isPrivateIPv4(ip)) }
        for ip in ["8.8.8.8","127.0.0.1","169.254.1.1","172.32.0.1","192.168.002.1","192.168.1.256","192.168.1.1.example.com","192.168.1.1:5400"] { #expect(!TVTarget.isPrivateIPv4(ip)) }
    }
}
