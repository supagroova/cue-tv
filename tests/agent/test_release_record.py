import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[2] / 'scripts/agent/release-record.py'
spec = importlib.util.spec_from_file_location('release_record', MODULE)
record = importlib.util.module_from_spec(spec)
spec.loader.exec_module(record)


class ReleaseRecordTests(unittest.TestCase):
    def test_inventory_records_nested_binary_bytes_and_detects_replacement(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / 'Frameworks/Player.framework/Player'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'\x00\x01player')
            before = record.inventory(root)
            self.assertEqual(before, {'Frameworks/Player.framework/Player': {
                'bytes': 8, 'sha256': hashlib.sha256(b'\x00\x01player').hexdigest()}})
            binary.write_bytes(b'\x00\x02player')
            self.assertNotEqual(before, record.inventory(root))

    def test_empty_or_symlinked_artifact_is_not_silently_accepted(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            with self.assertRaisesRegex(ValueError, 'Empty'):
                record.inventory(root)
            (root / 'escape').symlink_to('/etc/hosts')
            with self.assertRaisesRegex(ValueError, 'Symlink'):
                record.inventory(root)

    def test_source_change_invalidates_candidate(self):
        before = {'commit': 'old', 'clean': True, 'submodules': ['pinned']}
        record.require_unchanged(before, dict(before))
        for after in ({**before, 'commit': 'new'}, {**before, 'submodules': ['different']}):
            with self.assertRaisesRegex(ValueError, 'Source changed'):
                record.require_unchanged(before, after)
