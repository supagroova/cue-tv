import importlib.util
import pathlib
import subprocess
import tempfile
import unittest

MODULE = pathlib.Path(__file__).resolve().parents[2] / 'scripts/native/artifact-cache.py'

class NativeArtifactTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        spec = importlib.util.spec_from_file_location('artifacts', MODULE)
        self.cache = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.cache)
        self.cache.ROOT = pathlib.Path(self.directory.name)
        subprocess.run(['git', 'init', '-q', self.directory.name], check=True)
        self.source = self.cache.ROOT / 'native/cue-core/engine.rs'
        self.source.parent.mkdir(parents=True)
        self.source.write_text('original source')
        subprocess.run(['git', '-C', self.directory.name, 'add', 'native'], check=True)
        self.target = self.cache.ROOT / '.agent-work/native-target'
        for name, relative in self.cache.ARTIFACTS['torrent'].items():
            path = self.target / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'compiled fixture ' + name.encode())

    def test_exact_hit_restores_complete_artifacts(self):
        key = self.cache.fingerprint('torrent')
        self.cache.transfer('save', 'torrent', key)
        for relative in self.cache.ARTIFACTS['torrent'].values():
            (self.target / relative).unlink()
        self.cache.transfer('restore', 'torrent', key)
        self.assertEqual((self.target / 'debug/examples/seed').read_bytes(), b'compiled fixture seed')
        self.assertTrue((self.target / 'debug/examples/seed').stat().st_mode & 0o100)

    def test_source_change_invalidates_previous_artifacts(self):
        key = self.cache.fingerprint('torrent')
        self.cache.transfer('save', 'torrent', key)
        torrent = self.cache.ROOT / 'native/cue-torrent/engine.rs'
        torrent.parent.mkdir(parents=True, exist_ok=True)
        torrent.write_text('changed source')
        subprocess.run(['git', '-C', self.directory.name, 'add', 'native'], check=True)
        self.assertNotEqual(key, self.cache.fingerprint('torrent'))
        with self.assertRaisesRegex(ValueError, 'current sources'):
            self.cache.transfer('restore', 'torrent', key)

    def test_missing_or_corrupt_artifact_fails_before_install(self):
        key = self.cache.fingerprint('torrent')
        with self.assertRaises(FileNotFoundError):
            self.cache.transfer('restore', 'torrent', key)
        self.cache.transfer('save', 'torrent', key)
        (self.cache.ROOT / '.agent-work/native-artifacts/torrent/libcue_torrent.a').write_bytes(b'corrupt')
        core = self.target / 'debug/examples/seed'
        core.write_bytes(b'unchanged sentinel')
        with self.assertRaisesRegex(ValueError, 'Invalid native artifact'):
            self.cache.transfer('restore', 'torrent', key)
        self.assertEqual(core.read_bytes(), b'unchanged sentinel')

    def test_core_edit_preserves_torrent_partial_hit(self):
        core_key = self.cache.fingerprint('core')
        torrent_key = self.cache.fingerprint('torrent')
        self.cache.transfer('save', 'torrent', torrent_key)
        self.source.write_text('new core implementation')
        self.assertNotEqual(core_key, self.cache.fingerprint('core'))
        self.assertEqual(torrent_key, self.cache.fingerprint('torrent'))
        self.cache.transfer('restore', 'torrent', torrent_key)
        self.assertEqual((self.target / 'debug/examples/seed').read_bytes(), b'compiled fixture seed')
        self.assertFalse((self.target / 'aarch64-apple-tvos-sim/debug/libcue_core.a').exists())

    def test_shared_build_configuration_invalidates_both_components(self):
        before = {c: self.cache.fingerprint(c) for c in ['core', 'torrent']}
        path = self.cache.ROOT / 'apple-tv/project.yml'
        path.parent.mkdir()
        path.write_text('compiler settings changed')
        subprocess.run(['git', '-C', self.directory.name, 'add', 'apple-tv'], check=True)
        for component in before:
            self.assertNotEqual(before[component], self.cache.fingerprint(component))

    def test_torrent_lock_and_vendor_changes_do_not_invalidate_core(self):
        core_key = self.cache.fingerprint('core')
        torrent_key = self.cache.fingerprint('torrent')
        for relative in ['native/cue-torrent/Cargo.lock', 'native/cue-torrent/vendor/dualstack/src/lib.rs']:
            path = self.cache.ROOT / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('changed dependency input')
            subprocess.run(['git', '-C', self.directory.name, 'add', 'native'], check=True)
            self.assertEqual(core_key, self.cache.fingerprint('core'))
            self.assertNotEqual(torrent_key, self.cache.fingerprint('torrent'))
            torrent_key = self.cache.fingerprint('torrent')

    def test_core_restore_does_not_require_or_modify_torrent_outputs(self):
        key = self.cache.fingerprint('core')
        path = self.target / 'aarch64-apple-tvos-sim/debug/libcue_core.a'
        path.write_bytes(b'core only')
        self.cache.transfer('save', 'core', key)
        path.unlink()
        seed = self.target / 'debug/examples/seed'
        seed.write_bytes(b'new torrent build')
        self.cache.transfer('restore', 'core', key)
        self.assertEqual(path.read_bytes(), b'core only')
        self.assertEqual(seed.read_bytes(), b'new torrent build')
