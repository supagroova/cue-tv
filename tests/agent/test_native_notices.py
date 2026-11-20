import importlib.util
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[2] / 'scripts/agent/collect-native-notices.py'


class NativeNoticeTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('native_notices', MODULE)
        self.collector = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.collector)

    def test_build_dependencies_are_kept_but_dev_only_subtrees_are_not(self):
        def node(name, deps):
            return {'id': name, 'deps': [{'pkg': key, 'dep_kinds': [{'kind': kind}]} for key, kind in deps]}
        metadata = {'packages': [{'id': key} for key in ('app', 'runtime', 'build', 'dev', 'dev-child')],
                    'resolve': {'root': 'app', 'nodes': [node('app', [('runtime', None), ('build', 'build'), ('dev', 'dev')]),
                                                        node('runtime', []), node('build', [('runtime', None)]),
                                                        node('dev', [('dev-child', None)]), node('dev-child', [])]}}
        self.assertEqual({p['id'] for p in self.collector.reachable_packages(metadata)}, {'app', 'runtime', 'build'})

    def test_missing_notice_and_wrong_override_version_fail_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            package = {'name': 'fixture', 'version': '1.0.0', 'manifest_path': str(root / 'Cargo.toml'), 'source': 'registry+fixture'}
            with self.assertRaisesRegex(ValueError, 'No notice'):
                self.collector.notice_inputs(package, [], root)
            override = {'packages': ['fixture'], 'versions': {'fixture': '2.0.0'}, 'retained': 'LICENSE', 'sha256': 'bad'}
            with self.assertRaisesRegex(ValueError, 'version'):
                self.collector.notice_inputs(package, [override], root)

    def test_changed_retained_notice_is_not_accepted_by_stale_mapping(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'COPYING').write_text('changed copyright notice')
            package = {'name': 'fixture', 'version': '1.0.0', 'manifest_path': str(root / 'Cargo.toml'), 'source': 'registry+fixture'}
            override = {'packages': ['fixture'], 'versions': {'fixture': '1.0.0'}, 'resolved_sources': {'fixture': 'registry+fixture'}, 'retained': 'COPYING', 'sha256': 'bad'}
            with self.assertRaisesRegex(ValueError, 'checksum'):
                self.collector.notice_inputs(package, [override], root)

    def test_same_version_at_different_git_revision_requires_review(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            package = {'name': 'fixture', 'version': '1.0.0', 'manifest_path': str(root / 'Cargo.toml'), 'source': 'git+fixture#new'}
            override = {'packages': ['fixture'], 'versions': {'fixture': '1.0.0'}, 'resolved_sources': {'fixture': 'git+fixture#old'}}
            with self.assertRaisesRegex(ValueError, 'source mismatch'):
                self.collector.notice_inputs(package, [override], root)

    def test_nested_vendor_and_spdx_license_directory_texts_are_retained(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for name in ('LICENSE', 'third_party/codec/COPYRIGHT', 'LICENSES/MIT.txt', 'target/NOTICE'):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            package = {'name': 'fixture', 'version': '1.0.0', 'manifest_path': str(root / 'Cargo.toml')}
            paths = self.collector.notice_inputs(package, [], root)
            self.assertEqual({str(p.relative_to(root)) for p in paths},
                             {'LICENSE', 'third_party/codec/COPYRIGHT', 'LICENSES/MIT.txt'})
