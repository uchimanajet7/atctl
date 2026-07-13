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
bundle, or Homebrew Cask distribution target. A GUI distribution would require
a separate product specification and packaging contract.

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

Source repository releases publish one checksum file per archive. The current
artifact contract does not include an aggregate checksum manifest, provenance,
attestation, or SBOM metadata.

Release notes:

- The source repository release workflow extracts the matching released-version
  section from `CHANGELOG.md`.
- The released-version section heading must include the package version and a
  `YYYY-MM-DD` release date, for example `## 0.1.0 - 2026-07-05`.
- The release workflow fails before tag or GitHub Release publication when the
  tag version does not match `Cargo.toml` or when `CHANGELOG.md` does not
  contain a non-empty section for that version.
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

The workflow validates the Cargo version and any existing tag, runs the normal
Rust verification gate, builds the archive, creates the checksum, and extracts
release notes from `CHANGELOG.md` before starting publication. After those
steps succeed, the final GitHub CLI operation creates a missing tag at the
selected workflow commit, uploads the archive and checksum through a draft
release, and publishes the GitHub Release. If the requested tag already exists,
the workflow verifies it points to the selected workflow commit before the
build and again immediately before publication; it fails without moving,
overwriting, or deleting a mismatched tag.

The GitHub Web workflow is the only automatic source-release entry point.
Pushing a tag does not start the release workflow. An existing tag at the
selected commit may still be published by entering that tag in the Web
workflow.

A failure before the final GitHub publication operation creates neither a new
tag nor a GitHub Release. The final operation creates a draft, uploads both
assets, and then publishes it. If that GitHub operation itself fails, inspect
any resulting draft and tag before retrying. The workflow does not
automatically delete or move remote tags or releases during failure recovery.

Do not create the GitHub Release page manually before running this workflow;
the workflow owns tag creation when needed, asset upload, and release-note
population.

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

  depends_on "pkgconf" => :build
  depends_on "rust" => :build
  depends_on arch: :arm64
  depends_on "libusb"
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/atctl", "--version"
  end
end
```

Bottles are owned by the Homebrew tap repository. The source repository release
workflow does not publish or update tap Formulae or bottles. Formula updates and
bottle publication are explicit tap-repository operations performed after the
source release is available.

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

`Cargo.toml` uses an explicit `include` whitelist so the Cargo source package
contains only the source, maintained examples, and public package files needed
by consumers. The whitelist is limited to:

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

## Release Operator Workflow

Complete the Rust source repository release before updating Homebrew. The
prepared tap repository is a separate release surface; preparing the repository
does not publish a Formula or bottle for a source release.

### Source Repository

Use the GitHub Web UI release operation in **Source Repository Release
Artifacts**. A successful source release provides:

- A version tag that matches `Cargo.toml`.
- A GitHub Release populated from the matching `CHANGELOG.md` section.
- The Apple Silicon macOS archive and its checksum file.

The source release workflow does not update the Homebrew tap.

### Homebrew Tap Repository

After the source release is available, continue in
`https://github.com/uchimanajet7/homebrew-atctl`:

1. Open **Actions** and run **Update Formula PR** with the released source tag.
2. Review the generated `Formula/atctl.rb` change, including the source archive
   URL, SHA-256, platform restrictions, dependencies, and tap CI result.
3. When bottle publication is required, run **Publish Bottles** with the
   reviewed pull-request number and its expected head SHA.
4. Merge only the reviewed Formula and bottle metadata produced by the tap
   workflow.

The Formula update must keep the contract defined above: macOS on Apple
Silicon, `libusb` as a runtime dependency, Rust and `pkgconf` as source-build
dependencies, and a source-build fallback using the tagged source archive.
Bottle publication remains an explicit tap-repository action and is not a side
effect of the Rust source release.

References:

- https://github.com/uchimanajet7/homebrew-atctl
- https://docs.github.com/actions/managing-workflow-runs/manually-running-a-workflow
- https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap
- https://docs.brew.sh/Formula-Cookbook
- https://docs.brew.sh/Bottles

## Release Decisions

Packaging readiness requires the source repository release artifact contract
and the Homebrew tap Formula/bottle contract to be reviewed as separate release
surfaces. Direct-download promotion is outside the current distribution
contract and would require signing, notarization, Gatekeeper, quarantine, and
credential-handling decisions.

See [DECISIONS.md](DECISIONS.md) for the accepted distribution decisions.
