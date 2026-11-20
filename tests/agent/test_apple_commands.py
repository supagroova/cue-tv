"""Exercise the real command wrapper against the Xcode process boundary."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class AppleCommandTests(unittest.TestCase):
    def run_combined(self, xcode_exit=0, action="ci-test"):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            script = root / "scripts/agent/apple.sh"
            script.parent.mkdir(parents=True)
            shutil.copy2(ROOT / "scripts/agent/apple.sh", script)
            (root / "apple-tv/Config").mkdir(parents=True)
            (root / "apple-tv/TVConfig.example.plist").write_text("fixture")
            binary = root / "bin"
            binary.mkdir()
            (binary / "xcodegen").write_text("#!/bin/sh\nexit 0\n")
            (binary / "xcodebuild").write_text(
                f"#!{sys.executable}\nimport json, os, sys\n"
                "with open(os.environ['CUE_TEST_ARGS'], 'a') as out:\n"
                "    out.write(json.dumps(sys.argv[1:]) + '\\n')\n"
                "sys.exit(int(os.environ['CUE_TEST_EXIT']))\n"
            )
            for executable in binary.iterdir():
                executable.chmod(0o755)
            record = root / "commands.jsonl"
            environment = dict(os.environ, PATH=f"{binary}:{os.environ['PATH']}",
                               DESTINATION="platform=tvOS Simulator,id=fixture-device",
                               CUE_TEST_ARGS=str(record), CUE_TEST_EXIT=str(xcode_exit))
            if action == "release-build":
                environment.pop("DESTINATION", None)
            result = subprocess.run(["bash", str(script), action], env=environment,
                                    capture_output=True, text=True)
            commands = [json.loads(line) for line in record.read_text().splitlines()] if record.exists() else []
            return result, commands

    def test_combined_run_keeps_both_required_suites_in_one_xcode_invocation(self):
        result, commands = self.run_combined()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(commands), 1)
        arguments = commands[0]
        self.assertIn("-only-testing:SittingRoomTVTests", arguments)
        self.assertIn("-only-testing:SittingRoomTVUITests/PlaybackLabTests", arguments)
        self.assertNotIn("-only-testing:SittingRoomTVUITests", arguments)
        self.assertFalse(any("LongPlaybackTests" in argument for argument in arguments))
        self.assertIn("CODE_SIGNING_ALLOWED=YES", arguments)
        self.assertIn("AppIdentifierPrefix=CueTVSimulator.", arguments)
        self.assertEqual(arguments[-1], "test")

    def test_combined_run_propagates_xcode_failure(self):
        result, commands = self.run_combined(xcode_exit=7)
        self.assertEqual(len(commands), 1)
        self.assertEqual(result.returncode, 7)

    def test_release_build_uses_release_device_configuration_without_signing(self):
        result, commands = self.run_combined(action="release-build")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(commands), 1)
        arguments = commands[0]
        self.assertEqual(arguments[arguments.index("-configuration") + 1], "Release")
        self.assertEqual(arguments[arguments.index("-sdk") + 1], "appletvos")
        self.assertEqual(arguments[arguments.index("-destination") + 1], "generic/platform=tvOS")
        self.assertIn("CODE_SIGNING_ALLOWED=NO", arguments)
        self.assertEqual(arguments[-1], "build")
