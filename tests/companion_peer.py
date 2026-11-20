"""Local-only Companion interoperability peer for the Kotlin client. Uses the existing Python
codec and cryptography implementation, plus disposable deterministic test identities. No TV."""
import asyncio
from pathlib import Path
import sys
import time
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'bridge'))
import appletv as wire
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.asymmetric.x25519 import X25519PrivateKey, X25519PublicKey
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

mode = sys.argv[1] if len(sys.argv) > 1 else 'ok'

async def main():
    finished = asyncio.get_running_loop().create_future()
    async def peer(reader, writer):
        incoming = outgoing = None
        recv_n = send_n = 0
        async def recv(expected):
            nonlocal recv_n
            header = await reader.readexactly(4)
            assert header[0] == expected
            size = int.from_bytes(header[1:], 'big')
            assert 0 < size <= wire.MAX_FRAME
            data = await reader.readexactly(size)
            if incoming:
                data = wire.ChaCha20Poly1305(incoming).decrypt(recv_n.to_bytes(12, 'little'), data, header)
                recv_n += 1
            return wire.opack_decode(data)
        async def send(kind, value):
            nonlocal send_n
            data = wire.opack_encode(value)
            header = bytes([kind]) + (len(data) + (16 if outgoing else 0)).to_bytes(3, 'big')
            if outgoing:
                data = wire.ChaCha20Poly1305(outgoing).encrypt(send_n.to_bytes(12, 'little'), data, header)
                send_n += 1
            writer.write(header + data)
            await writer.drain()
        try:
            first = await recv(5)
            assert first['_auTy'] == 4
            tlv = wire.tlv_decode(first['_pd'])
            assert tlv[6] == b'\x01'
            client_public = tlv[3]
            ephemeral = X25519PrivateKey.generate()
            public = wire.raw_public(ephemeral)
            shared = ephemeral.exchange(X25519PublicKey.from_public_bytes(client_public))
            key = wire.hkdf(shared, b'Pair-Verify-Encrypt-Salt', b'Pair-Verify-Encrypt-Info')
            server = Ed25519PrivateKey.from_private_bytes(bytes(range(32, 64)))
            identity = b'wrong-device' if mode == 'identity' else b'test-apple-tv'
            signature = server.sign(public + identity + client_public)
            if mode == 'signature': signature = bytes([signature[0] ^ 1]) + signature[1:]
            inner = wire.tlv_encode([(1, identity), (10, signature)])
            await send(6, {'_pd': wire.tlv_encode([(6, b'\x02'), (3, public), (5, wire.seal(key, b'PV-Msg02', inner))])})
            if mode in ('signature', 'identity'):
                assert await reader.read() == b''
                finished.set_result(True)
                return
            second = await recv(6)
            tlv = wire.tlv_decode(second['_pd'])
            assert tlv[6] == b'\x03'
            inner = wire.tlv_decode(wire.unseal(key, b'PV-Msg03', tlv[5]))
            assert inner[1] == b'test-remote'
            Ed25519PrivateKey.from_private_bytes(bytes(range(32))).public_key().verify(inner[10], client_public + inner[1] + public)
            await send(6, {'_pd': wire.tlv_encode([(6, b'\x04')])})
            incoming = wire.hkdf(shared, b'', b'ClientEncrypt-main')
            outgoing = wire.hkdf(shared, b'', b'ServerEncrypt-main')
            buttons = mode == 'buttons'
            events = ['_systemInfo', '_sessionStart', 'TVRCSessionStart']
            if buttons:
                events += ['_hidC'] * 8
            else:
                events += ['FetchAttentionState']
                if mode != 'awake': events += ['_hidC', '_hidC']
                events += ['_launchApp']
            hid = 0
            pressed_at = None
            for event in events:
                message = await recv(8)
                assert message['_i'] == event and message['_t'] == 2
                if event == '_hidC':
                    code = [6, 6, 7, 7][hid // 2] if buttons else 5
                    state = hid % 2 + 1 if buttons else hid + 1
                    assert message['_c'] == {'_hBtS': state, '_hidC': code}
                    if state == 1: pressed_at = time.monotonic()
                    elif buttons:
                        elapsed = time.monotonic() - pressed_at
                        assert elapsed >= .95 if hid // 2 in (1, 3) else elapsed < .9
                    hid += 1
                if event == '_launchApp':
                    assert message['_c'] == {'_urlS': 'stremio:///detail/series/tt4574334'}
                    await send(8, {'_i': '_event', '_t': 1, '_c': {'ready': True}})
                content = {'_sid': 123} if event == '_sessionStart' else {'state': 3 if mode == 'awake' else 2} if event == 'FetchAttentionState' else {}
                await send(8, {'_x': message['_x'], '_t': 3, '_c': content})
            finished.set_result(True)
        except Exception as exc:
            if not finished.done(): finished.set_exception(exc)
        finally:
            writer.close()
            await writer.wait_closed()
    async with await asyncio.start_server(peer, '127.0.0.1', 0) as server:
        print(server.sockets[0].getsockname()[1], flush=True)
        await asyncio.wait_for(finished, 20)
        print('PASS', flush=True)

asyncio.run(main())
