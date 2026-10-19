# Making a QuipIt release

You can do this in a browser; no terminal commands or Rust installation are needed.

## Build the downloads

1. Open [QuipIt's Actions page](https://github.com/TriLogic/QuipIt/actions).
2. Choose **Build draft release** in the left sidebar.
3. Click **Run workflow**, leave the branch as **main**, and click the green
   **Run workflow** button.
4. Open the new run to watch its progress. The first run can take a while because
   GitHub builds the application and its dependencies on four separate machines.
5. When all jobs turn green, open [Releases](https://github.com/TriLogic/QuipIt/releases).
   Sign in with repository write access to see the new **draft**.

The version comes from `Cargo.toml`. The current version is **v0.3.1**. Nothing is
published automatically. A failure in any platform's tests or packaging prevents
creation of the draft; the successful build downloads remain available in the run's
**Artifacts** section for 14 days.

## Test and publish

Download the matching package from the draft's Assets:

| Your computer | Download | Run it |
| --- | --- | --- |
| Windows, Intel/AMD 64-bit | `QuipIt-VERSION-windows-x64.zip` | Extract the ZIP, then open `quipit.exe`. |
| Mac, Apple Silicon (M-series) | `QuipIt-VERSION-macos-apple-silicon.zip` | Extract, then move `QuipIt.app` to Applications. |
| Mac, Intel | `QuipIt-VERSION-macos-intel.zip` | Extract, then move `QuipIt.app` to Applications. |
| Linux, Intel/AMD 64-bit | `QuipIt-VERSION-linux-x64.tar.gz` | Extract, then run `./quipit` in a graphical desktop session. |

The Windows executable requires the [Microsoft Visual C++ v14 x64 runtime](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist).
If Windows reports a missing `VCRUNTIME140.dll`, install that runtime first.

These are initial portable packages, not installers. Windows packages have no
publisher signature. Mac apps are ad-hoc signed, without an Apple Developer ID or
notarization, so the operating system may block them. On a Mac, use System Settings
→ Privacy & Security → Open Anyway for a downloaded build you trust
([Apple’s instructions](https://support.apple.com/en-us/102445)). Linux builds
use Ubuntu 22.04 and depend on system desktop/graphics libraries; the package
includes `runtime-libraries.txt` listing its linked libraries. Other Linux
distributions may need corresponding packages installed. No `.qip` file
association is installed yet; open projects through QuipIt's File → Open menu.

Test opening images, adding/editing/reordering objects, saving/reopening a `.qip`,
and exporting an image on each platform. The workflow's unit tests cannot replace
these graphical desktop checks.

When satisfied, edit the draft's notes and click **Publish release**. The downloads
then appear on the public Releases page. `SHA256SUMS.txt` contains checksums for all
four downloads.

## Later versions and retries

- Change `version` in `Cargo.toml`, run `cargo check` to update `Cargo.lock`, update
  `CHANGELOG.md`, and commit/push before running the workflow again.
- Pushing a matching version tag such as `v0.1.1` also starts the same draft workflow.
- Use **Re-run failed jobs** if a build fails. Re-running for the same commit can
  replace assets on its existing draft.
- The workflow refuses to overwrite a published release or reuse a draft version
  for a different commit. Delete an unpublished draft before retrying that version
  from changed code, or choose a new version.
- Apple Developer ID signing/notarization, Windows signing, custom app icons,
  native installers, and Linux dependency bundling can be added separately.
