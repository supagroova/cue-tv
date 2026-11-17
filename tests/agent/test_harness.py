import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts/agent' / (name + '.py'))
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded

check = module('check')
task = module('task')

class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
    def tearDown(self):
        self.temp.cleanup()
    def test_guard_distinguishes_private_config_from_example(self):
        for name in ['.env', '.env.local', 'apple-tv/Config/Signing.mine.xcconfig', 'cert.p12', 'apple-tv/TVConfig.plist']:
            self.assertTrue(check.sensitive(name), name)
        for name in ['.env.example', 'apple-tv/Config/Signing.example.xcconfig', 'apple-tv/Sources/TVApp.swift']:
            self.assertFalse(check.sensitive(name), name)
    def test_actual_staged_guard_rejects_private_file(self):
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        (self.root / 'safe.swift').write_text('struct Example {}')
        (self.root / '.env').write_text('SYNTHETIC=not-a-secret')
        subprocess.run(['git', '-C', str(self.root), 'add', 'safe.swift'], check=True)
        self.assertEqual(check.staged(self.root), [])
        subprocess.run(['git', '-C', str(self.root), 'add', '.env'], check=True)
        self.assertEqual(check.staged(self.root), ['keep local: .env'])
    def test_failed_verification_is_retained_and_not_completed(self):
        task.start(self.root, 'proof')
        result = task.verify(self.root, 'proof', [sys.executable, '-c', 'print("failure evidence"); raise SystemExit(7)'])
        self.assertEqual(result['exit_code'], 7)
        self.assertEqual(result['result'], 'fail')
        self.assertIn('failure evidence', (self.root / result['log']).read_text())
        result2 = task.verify(self.root, 'proof', [sys.executable, '-c', 'print("ok")'])
        self.assertEqual(result2['result'], 'pass')
        self.assertNotEqual(result['log'], result2['log'])
        self.assertEqual(len(list((self.root / '.agent-work/tasks/proof').glob('*.log'))), 2)
    def test_identity_detects_source_change_but_not_logs(self):
        source = self.root / 'source.swift'
        source.write_text('one')
        before = task.identity(self.root)
        task.start(self.root, 'proof')
        self.assertEqual(task.identity(self.root), before)
        source.write_text('two')
        self.assertNotEqual(task.identity(self.root), before)
    def test_task_traversal_and_duplicate_start_rejected(self):
        with self.assertRaises(ValueError): task.start(self.root, '../escape')
        task.start(self.root, 'proof')
        with self.assertRaises(FileExistsError): task.start(self.root, 'proof')
        with self.assertRaises(ValueError): task.verify(self.root, 'missing', ['true'])
    def test_missing_command_is_failed_evidence(self):
        task.start(self.root, 'proof')
        result = task.verify(self.root, 'proof', ['/nonexistent/cue-test-command'])
        self.assertEqual(result['exit_code'], 127)
        self.assertEqual(result['result'], 'fail')
    def test_broken_local_link_and_alias_are_reported(self):
        (self.root / 'AGENTS.md').write_text('[missing](absent.md)')
        (self.root / 'CLAUDE.md').write_text('@AGENTS.md')
        skill = self.root / '.agents/skills/example/SKILL.md'
        skill.parent.mkdir(parents=True)
        skill.write_text('---\nname: example\ndescription: Example\n---\n')
        findings = check.check(self.root)
        self.assertTrue(any('absent.md' in x for x in findings))
        self.assertTrue(any('symlink' in x for x in findings))

    def test_real_git_hook_blocks_sensitive_commit(self):
        import shutil
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        for relative in ['scripts/agent/check.py', '.githooks/pre-commit']:
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / relative, target)
        subprocess.run(['git', '-C', str(self.root), 'config', 'core.hooksPath', '.githooks'], check=True)
        private = self.root / 'Signing.mine.xcconfig'
        private.write_text('SYNTHETIC = not-a-secret')
        subprocess.run(['git', '-C', str(self.root), 'add', str(private)], check=True)
        result = subprocess.run(['git', '-C', str(self.root), '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-m', 'synthetic fixture'], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('keep local:', result.stdout + result.stderr)
    def test_identity_excludes_private_settings(self):
        p = self.root / 'Signing.mine.xcconfig'
        p.write_text('SYNTHETIC = first')
        first = task.identity(self.root)
        p.write_text('SYNTHETIC = second')
        self.assertEqual(task.identity(self.root), first)
    def test_identity_tracks_symlink_target_changes(self):
        p = self.root / 'alias'
        p.symlink_to('first')
        first = task.identity(self.root)
        p.unlink()
        p.symlink_to('second')
        self.assertNotEqual(task.identity(self.root), first)

if __name__ == '__main__': unittest.main()
