# Packaging

This document defines the packaging contract for `atctl`.

## Packaged Platform Target

Packaged platform target:

```text
macOS Apple Silicon arm64
```

Other platform artifacts must not be promised until their packaging path is
validated:

- macOS Intel x86_64
- Linux x86_64
- Linux arm64
- Windows
- Universal macOS binaries

## Product Distribution Classification

`atctl` is a CLI/TUI executable. It is not a macOS GUI application, app
bundle, or Homebrew Cask distribution target unless a later approved product
specification adds a GUI application.

End users install the command through Homebrew. GitHub Releases
artifacts are release outputs and manual artifacts; they are not the normal
install path.

## Repository Responsibilities

Release and Homebrew materials must be kept separate.

```text
Source repository: https://github.com/uchimanajet7/atctl
  Owns source code, specifications, local development docs, CI verification,
  release workflows, GitHub Releases assets, and checksums.

Homebrew tap repository: https://github.com/uchimanajet7/homebrew-atctl
  Owns Formula/atctl.rb, tap-specific CI, tap metadata, and bottle metadata.
```

Local development builds, CI verification builds, source repository release
builds, and Homebrew installation behavior are separate concerns. Documentation
and implementation must not treat one of these as a decision for the others.
Source repository release creation and Homebrew publication are also separate
release operations: a GitHub Release may be created without updating Homebrew,
and a Homebrew publication must be an explicit tap-repository action.

## Homebrew Install Flow

Normal user flow:

```sh
brew install uchimanajet7/atctl/atctl
```

The source repository and Homebrew tap repository are separate:

```text
Source repository: https://github.com/uchimanajet7/atctl
Homebrew tap repository: https://github.com/uchimanajet7/homebrew-atctl
User-facing tap name: uchimanajet7/atctl
```

Homebrew's one-argument GitHub tap form maps `brew tap <user>/<repo>` to
`https://github.com/<user>/homebrew-<repo>`. Because of that convention,
`brew tap uchimanajet7/atctl` clones `uchimanajet7/homebrew-atctl`; it does not
clone the source repository `uchimanajet7/atctl`.

The fully qualified install form `brew install uchimanajet7/atctl/atctl`
selects the `atctl` formula from that tap without requiring a separate visible
tap step. The equivalent tapped form remains:

```sh
brew tap uchimanajet7/atctl
brew install atctl
```

References:

- https://docs.brew.sh/Taps
- https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap

## Source Repository Release Artifacts

The source repository must use GitHub Actions to build release artifacts and
upload them to GitHub Releases for source repository releases. That release
workflow is independent from the Homebrew tap decision.

Release artifacts:

- Apple Silicon macOS archive asset:
  `atctl-v{VERSION}-aarch64-apple-darwin.tar.gz`
- Checksum asset:
  `atctl-v{VERSION}-aarch64-apple-darwin.tar.gz.sha256`

`{VERSION}` is the semantic version without the leading `v`. The
`aarch64-apple-darwin` suffix identifies the Rust/macOS Apple Silicon target.
This naming is a common Rust CLI release-asset convention, not a formal GitHub
or Rust standard.

Checksum content:

```text
<sha256 hex>  atctl-v{VERSION}-aarch64-apple-darwin.tar.gz
```

Source repository releases publish one checksum file per archive. They do not
publish an aggregate checksum manifest, provenance, attestation, or SBOM
metadata unless those metadata types are separately approved before being
promised or implemented.

Release notes:

- The source repository release workflow extracts the matching released-version
  section from `CHANGELOG.md`.
- The released-version section heading must include the package version and a
  `YYYY-MM-DD` release date, for example `## 0.1.0 - 2026-07-05`.
- The release workflow fails before GitHub Release creation when the tag
  version does not match `Cargo.toml` or when `CHANGELOG.md` does not contain a
  non-empty section for that version.
- GitHub automatically generated release notes are not the primary release-note
  source for this project because they summarize merged pull requests and
  contributors, while the project needs curated user-facing notes from
  `CHANGELOG.md`.

GitHub Web UI release operation:

1. Open the source repository on GitHub.
2. Open **Actions**.
3. Select the **Release** workflow.
4. Select **Run workflow**.
5. Select the branch or commit that should be released.
6. Enter `release_tag`, for example `v0.1.0`.
7. Run the workflow.

The workflow creates the requested tag at the selected workflow commit when the
tag does not already exist. If the tag already exists, the workflow verifies
that it points to the selected workflow commit and fails without moving the tag
when it does not. The same workflow run validates the Cargo version, builds the
archive, creates the checksum, extracts release notes from `CHANGELOG.md`, and
creates the GitHub Release. Do not create the GitHub Release page manually
before running this workflow; the workflow owns GitHub Release creation and
release-note population.

A pushed tag matching `v*.*.*` remains a valid release trigger. Both trigger
paths use the same release validation, artifact packaging, checksum creation,
and changelog-backed release-note extraction.

References:

- https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases
- https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository
- https://docs.github.com/actions/managing-workflow-runs/manually-running-a-workflow
- https://docs.github.com/actions/using-workflows/triggering-a-workflow
- https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes
- https://docs.github.com/en/rest/releases/assets
- https://cli.github.com/manual/gh_release_create
- https://keepachangelog.com/en/1.1.0/
- https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html
- https://github.com/sharkdp/bat/releases
- https://github.com/BurntSushi/ripgrep/releases
- https://github.com/rust-lang/rust-bindgen/releases

## Homebrew Formula and Bottle Contract

The normal Homebrew distribution uses the tap formula. The preferred normal
state is a bottle-backed formula for each packaged platform target. This keeps
the normal end-user install command as
`brew install uchimanajet7/atctl/atctl`, or `brew install atctl` after tapping
`uchimanajet7/atctl`, while avoiding a Rust build on the user's machine when a
matching bottle is available.

The formula must keep source-build support as a fallback. Source builds are
used when no matching bottle is available, when the user disables bottle use,
or when maintainers verify the formula from source.

The source-build fallback must build from the `uchimanajet7/atctl` source
repository release archive. The formula should follow this contract:

```ruby
class Atctl < Formula
  desc "CLI/TUI AT command controller for USB cellular modems"
  homepage "https://github.com/uchimanajet7/atctl"
  url "https://github.com/uchimanajet7/atctl/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "<source archive sha256>"
  license "MIT"

  depends_on "rust" => :build
  depends_on "pkgconf" => :build
  depends_on "libusb"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/atctl", "--version"
  end
end
```

Bottles are owned by the Homebrew tap repository. The source repository release
workflow must not silently publish or update tap bottles. If cross-repository
automation is later introduced, the exact token storage, write permissions, and
update behavior must be explicitly approved first.

For bottled installs:

- `libusb` remains a runtime dependency.
- Rust is not a runtime dependency.
- The formula and user-facing docs must not imply that end users need Cargo.

The Homebrew tap must not install the GitHub Releases `.tar.gz` artifact as the
normal Homebrew path. GitHub Releases artifacts and Homebrew bottles are
separate release outputs with separate ownership and verification.

## Cargo Source Package Contract

The Cargo source package is Rust package metadata and `.crate` source-package
output. It is not the normal end-user install path. End users install
through Homebrew, and the Homebrew formula source-build fallback remains a
separate packaging concern.

`Cargo.toml` uses an explicit `include` whitelist so project-local agent files,
backups, local history, build outputs, and release-workflow drafts cannot be
accidentally included in Cargo package output. The whitelist is limited to:

- `src/**`
- `examples/presets/**`
- `examples/sequences/**`
- `README.md`
- `CHANGELOG.md`
- `LICENSE`

The repository-managed examples are included because current source and tests
load them directly. Repository documentation under `docs/**` remains source
repository documentation and is not included in the Cargo source package only
because `README.md` links to it. README links to repository documentation should
use GitHub URLs so the links remain useful when the README is rendered outside
the source repository, such as on crates.io or docs.rs.

Cargo package verification must include at least:

```sh
cargo package --list --allow-dirty
cargo package --allow-dirty
```

The first command confirms the file set. The second command creates the package
and lets Cargo extract and build it from a pristine package copy.

References:

- https://doc.rust-lang.org/cargo/reference/manifest.html
- https://doc.rust-lang.org/cargo/commands/cargo-package.html
- https://doc.rust-lang.org/cargo/reference/publishing.html

## Direct Download, Signing, and Notarization

Direct GitHub Releases download is not the normal end-user install path.

Developer ID signing and Apple notarization are not required for the normal
Homebrew install path.

GitHub Releases prebuilt `.tar.gz` artifacts may be published without Developer
ID signing and Apple notarization while they remain release/manual artifacts.
Documentation must not present direct GitHub Releases download as the normal
end-user install path.

If direct GitHub Releases download is promoted to a normal end-user install path,
Developer ID signing, Apple notarization, Gatekeeper behavior, quarantine
warnings, Apple Developer credentials, certificate handling, CI secret
management, and Apple notary service availability must be decided before that
promotion.

Unsigned or unnotarized direct-download macOS binaries may trigger Gatekeeper or
quarantine warnings depending on how the user downloads and runs them.

References:

- https://docs.brew.sh/Formula-Cookbook
- https://docs.brew.sh/Bottles
- https://docs.brew.sh/Cask-Cookbook
- https://developer.apple.com/developer-id/
- https://developer.apple.com/documentation/security/customizing-the-notarization-workflow
- https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution

## Dependency Notes

- `libusb` is the native runtime library.
- `pkgconf` provides the `pkg-config` command used during source builds.
- Rust is needed for source builds only.

## Final-Phase Release and Homebrew Workflow Plan

This section records the final-phase packaging guidance and current release
workflow boundary. It applies after the application features have been
implemented and approved. Source repository release artifact automation is now
present; Homebrew tap formula and bottle automation remain separate pending
tap-repository work.

Before final packaging approval, the implementation was not allowed to create:

- `.github/workflows/release.yml` in the source repository.
- `Formula/atctl.rb` in the Homebrew tap repository.
- Tap repository CI workflows.
- GitHub Releases, release assets, tags, bottles, or cross-repository writes.

### Source Repository Release Workflow

The source repository release workflow is:

```text
.github/workflows/release.yml
```

The release workflow uses:

- Triggers: pushed tags matching `v*.*.*`, and manual GitHub Actions
  `workflow_dispatch` runs with a required `release_tag` input.
- Runner: `macos-26`, which current GitHub-hosted runner documentation and
  GitHub Changelog list as a standard arm64 macOS runner. This avoids the
  moving `macos-latest` label for release builds.
- Token permissions: `contents: write` for the release job because GitHub
  Actions documents that this permission allows creating a release. Other
  permissions should remain unset or read-only unless a later approved workflow
  step requires them.
- Actions: first-party `actions/checkout` only. Release creation should use the
  GitHub CLI already available on the runner instead of a third-party release
  action.
- Rust target: `aarch64-apple-darwin`.

The workflow steps are:

1. Check out the source repository.
2. Validate the release version:
   - The release tag must start with `v`.
   - The tag version without the leading `v` must match `Cargo.toml`
     `package.version`.
3. For manual GitHub Actions runs, prepare the release tag:
   - Create the requested tag at the selected workflow commit when the tag does
     not exist.
   - Verify that an existing tag points to the selected workflow commit.
   - Fail without moving or overwriting an existing tag that points to another
     commit.
4. Install or confirm build dependencies:
   - Rust toolchain with `rustfmt` and `clippy`.
   - `libusb`.
   - `pkgconf`.
5. Run:

   ```sh
   cargo fmt --check
   cargo check --all-targets --all-features --locked
   cargo test --all-features --locked
   cargo clippy --all-targets --all-features --locked -- -D warnings
   cargo build --release --locked --target aarch64-apple-darwin
   ```

6. Stage the release archive:

   ```text
   dist/atctl-v{VERSION}-aarch64-apple-darwin.tar.gz
   ```

   The archive contains the `atctl` executable at its top level.

7. Generate the checksum:

   ```text
   dist/atctl-v{VERSION}-aarch64-apple-darwin.tar.gz.sha256
   ```

8. Extract the matching released-version section from `CHANGELOG.md` into:

   ```text
   dist/release-notes.md
   ```

   The extraction must fail before release creation if the section is missing,
   has no content beyond the heading, or lacks a `YYYY-MM-DD` release date.

9. Create the GitHub Release with:

   ```sh
   gh release create "$TAG" \
     "dist/atctl-v{VERSION}-aarch64-apple-darwin.tar.gz" \
     "dist/atctl-v{VERSION}-aarch64-apple-darwin.tar.gz.sha256" \
     --verify-tag \
     --title "atctl $TAG" \
     --notes-file "dist/release-notes.md"
   ```

If a release already exists and assets need to be added manually, use
`gh release upload <tag> <files>...`. Do not use `gh release upload --clobber`
unless replacement of already-published assets has been separately approved,
because GitHub CLI documents that `--clobber` deletes existing assets before
re-uploading them.

The source repository workflow must not:

- Update the Homebrew tap repository.
- Create or update Homebrew Formula pull requests.
- Trigger Homebrew publication automatically.
- Publish Homebrew bottles.
- Publish provenance, attestation, or SBOM files.
- Sign or notarize the `.tar.gz` artifact.
- Add Linux, Intel Mac, Windows, or universal binary release artifacts.

### Homebrew Tap Repository Workflow

After final packaging approval, Homebrew material must be implemented in:

```text
https://github.com/uchimanajet7/homebrew-atctl
```

User-facing install command:

```sh
brew install uchimanajet7/atctl/atctl
```

The tap repository implementation should add:

```text
Formula/atctl.rb
```

The tap formula must build from the tagged source archive in
`uchimanajet7/atctl` when source-build fallback is used. It must not install the
prebuilt GitHub Releases `.tar.gz` artifact as the normal Homebrew path.

The tap formula should follow this source-build fallback contract:

```ruby
class Atctl < Formula
  desc "CLI/TUI AT command controller for USB cellular modems"
  homepage "https://github.com/uchimanajet7/atctl"
  url "https://github.com/uchimanajet7/atctl/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "<source archive sha256>"
  license "MIT"

  depends_on "rust" => :build
  depends_on "pkgconf" => :build
  depends_on "libusb"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/atctl", "--version"
  end
end
```

If `brew tap-new` is used to initialize the tap repository, generated default
workflow files must be reviewed before commit. Homebrew documentation states
that leaving the default workflow files in place can build and upload bottles.
Because bottles are part of the intended normal Homebrew distribution, those
workflows must be reviewed as product release automation rather than accepted
as incidental generated files.

Formula update automation must be implemented as tap repository work, not as a
hidden side effect of the source repository release workflow. The intended
operator path is a manually triggered tap workflow in
`uchimanajet7/homebrew-atctl`, for example
`.github/workflows/update-formula-pr.yml`, using `workflow_dispatch` with the
approved source release tag as input.

That tap workflow should:

1. Read the requested source release tag, such as `v0.1.0`.
2. Resolve the `uchimanajet7/atctl` source archive URL and SHA-256.
3. Update `Formula/atctl.rb`.
4. Create or update a pull request in `uchimanajet7/homebrew-atctl`.

The source repository release workflow must not trigger this automatically.
This keeps release builds and Homebrew publication independently executable, so
an `atctl` GitHub Release can be produced without publishing that version to
Homebrew.

The tap repository work must:

- Publish bottles only through reviewed tap repository automation.
- Keep `libusb` as a runtime dependency.
- Keep Rust and `pkgconf` as source-build dependencies, not runtime
  dependencies.
- Avoid presenting Cargo or Rust installation as normal end-user prerequisites
  when a bottle is available.
- Avoid installing the GitHub Releases prebuilt `.tar.gz` artifact as the
  normal Homebrew path.
- Avoid adding a tap README or extra tap metadata unless separately approved.
- Avoid adding cross-repository automation secrets unless separately approved.

### Implementation Order

The implementation order after final packaging approval should be:

1. Add source repository release workflow.
2. Verify the workflow file locally as far as possible without publishing a
   release.
3. Request approval before creating or pushing any release tag.
4. Implement tap repository formula material separately in
   `uchimanajet7/homebrew-atctl`.
5. Implement a tap-side manual Formula update workflow, such as
   `.github/workflows/update-formula-pr.yml`, that creates or updates a
   Formula pull request only when an operator runs it for a chosen release tag.
6. Implement and review tap repository bottle automation as release automation,
   not as incidental generated workflow output.
7. Verify the formula with Homebrew source-build checks and bottle-path checks
   before claiming the Homebrew installation path is ready.

References:

- https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
- https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- https://github.blog/changelog/2026-02-26-macos-26-is-now-generally-available-for-github-hosted-runners/
- https://cli.github.com/manual/gh_release_create
- https://cli.github.com/manual/gh_release_upload
- https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes
- https://keepachangelog.com/en/1.1.0/
- https://docs.brew.sh/Taps
- https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap
- https://docs.brew.sh/Formula-Cookbook
- https://docs.brew.sh/Bottles
- https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html

## Release Blocking Decisions

Packaging readiness requires the source repository release artifact plan and
the Homebrew tap formula/bottle plan to be reviewed as separate release
surfaces. Direct-download promotion remains out of scope unless separately
approved with signing, notarization, Gatekeeper, quarantine, and credential
handling decisions.

See [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md).
