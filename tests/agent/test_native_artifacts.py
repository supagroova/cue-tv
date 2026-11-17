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
        self.source = self.cache.ROOT / 'native/engine.rs'
        self.source.parent.mkdir()
        self.source.write_text('original source')
        subprocess.run(['git', '-C', self.directory.name, 'add', 'native'], check=True)
        self.target = self.cache.ROOT / '.agent-work/native-target'
        for name, relative in self.cache.ARTIFACTS.items():
            path = self.target / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'compiled fixture ' + name.encode())

    def test_exact_hit_restores_complete_artifacts(self):
        key = self.cache.fingerprint()
        self.cache.transfer('save', key)
        for relative in self.cache.ARTIFACTS.values():
            (self.target / relative).unlink()
        self.cache.transfer('restore', key)
        self.assertEqual((self.target / 'debug/examples/seed').read_bytes(), b'compiled fixture seed')
        self.assertTrue((self.target / 'debug/examples/seed').stat().st_mode & 0o100)

    def test_source_change_invalidates_previous_artifacts(self):
        key = self.cache.fingerprint()
        self.cache.transfer('save', key)
        self.source.write_text('changed source')
        self.assertNotEqual(key, self.cache.fingerprint())
        with self.assertRaisesRegex(ValueError, 'current sources'):
            self.cache.transfer('restore', key)

    def test_missing_or_corrupt_artifact_fails_before_install(self):
        key = self.cache.fingerprint()
        with self.assertRaises(FileNotFoundError):
            self.cache.transfer('restore', key)
        self.cache.transfer('save', key)
        (self.cache.ROOT / '.agent-work/native-artifacts/libcue_torrent.a').write_bytes(b'corrupt')
        core = self.target / 'aarch64-apple-tvos-sim/debug/libcue_core.a'
        core.write_bytes(b'unchanged sentinel')
        with self.assertRaisesRegex(ValueError, 'Invalid native artifact'):
            self.cache.transfer('restore', key)
        self.assertEqual(core.read_bytes(), b'unchanged sentinel')
