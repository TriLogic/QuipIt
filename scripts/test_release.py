import json
import os
from pathlib import Path
import plistlib
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import release


class ReleaseTests(unittest.TestCase):
    def test_archives_contain_binaries_licenses_and_mac_bundle_metadata(self):
        for target in release.TARGETS:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                binary = root / "binary"
                binary.write_bytes(b"test executable")
                output = root / "dist"
                with patch.object(release.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "libc.so.6 => /lib/libc.so.6\n")):
                    release.package(target, binary=binary, output=output)
                archives = list(output.iterdir())
                self.assertEqual(len(archives), 1)
                if target == "linux-x64":
                    with tarfile.open(archives[0]) as archive:
                        executable = next(m for m in archive.getmembers() if m.name.endswith("/quipit"))
                        self.assertTrue(executable.mode & 0o111)
                        self.assertEqual(archive.extractfile(executable).read(), b"test executable")
                        self.assertTrue(any(m.name.endswith("runtime-libraries.txt") for m in archive.getmembers()))
                else:
                    with zipfile.ZipFile(archives[0]) as archive:
                        names = archive.namelist()
                        self.assertTrue(any(n.endswith("licenses/fonts/Ubuntu-UFL.txt") for n in names))
                        if target.startswith("macos"):
                            plist = plistlib.loads(archive.read(next(n for n in names if n.endswith("Info.plist"))))
                            self.assertEqual(plist["CFBundleExecutable"], "quipit")
                            self.assertEqual(plist["CFBundleShortVersionString"], release.version())
                            self.assertTrue(any("Contents/Resources/licenses/" in n for n in names))
                            info = archive.getinfo(next(n for n in names if n.endswith("MacOS/quipit")))
                            self.assertTrue((info.external_attr >> 16) & 0o111)
                        else:
                            self.assertEqual(archive.read(next(n for n in names if n.endswith("quipit.exe"))), b"test executable")

    def test_plan_rejects_tag_that_disagrees_with_cargo_version(self):
        with patch.dict(os.environ, {"GITHUB_REF_TYPE": "tag", "GITHUB_REF_NAME": "v999.0.0"}):
            with self.assertRaises(ValueError):
                release.plan()

    def test_missing_platform_cannot_create_a_draft(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(release, "ROOT", Path(temporary)), patch.object(release, "version", return_value="0.1.0"):
            with patch.object(release.subprocess, "run") as command:
                with self.assertRaises(RuntimeError):
                    release.publish()
                command.assert_not_called()

    def test_published_release_is_not_modified(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "dist").mkdir()
            (root / "CHANGELOG.md").write_text("Changes")
            for target in release.TARGETS:
                extension = ".tar.gz" if target == "linux-x64" else ".zip"
                (root / "dist" / f"QuipIt-0.1.0-{target}{extension}").write_bytes(b"test")
            result = subprocess.CompletedProcess([], 0, json.dumps({"isDraft": False, "targetCommitish": "abc", "url": "test"}))
            with patch.object(release, "ROOT", root), patch.object(release, "version", return_value="0.1.0"), patch.dict(os.environ, {"GITHUB_SHA": "abc"}), patch.object(release.subprocess, "run", return_value=result) as command:
                with self.assertRaises(RuntimeError):
                    release.publish()
                self.assertEqual(command.call_count, 1)


if __name__ == "__main__":
    unittest.main()
