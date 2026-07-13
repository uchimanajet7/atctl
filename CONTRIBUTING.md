# Contributing

Thank you for contributing to `atctl`. Pull requests are welcome for source
code, documentation, tests, and repository-managed examples.

## Before You Start

Search the existing issues and pull requests before starting work.

Small documentation corrections and clear bug fixes may be submitted directly
as pull requests. Open an issue and agree on the direction before implementing
any of the following:

- New features
- Product behavior or safety changes
- Release or distribution changes
- Large refactors
- Other changes with substantial user or maintenance impact

Keep issue and pull request discussions respectful, constructive, and focused
on the project.

## Development

Use the [development guide](docs/DEVELOPMENT.md) for the supported development
environment, local setup, and project commands. Use the
[product and technical specification](docs/SPEC.md) as the normative source for
product behavior and verification requirements.

## Verification

Before opening a pull request, run the normal Rust verification gate documented
in the [development guide](docs/DEVELOPMENT.md#format-and-lint). It covers
formatting, compilation checks, tests, and Clippy with warnings denied.

Run any additional change-specific checks described in the development guide.
When a change affects hardware-dependent behavior, run the relevant real-device
checks when possible. In the pull request, list both the checks that were run
and any relevant checks that were not run, including the reason.

## Pull Requests

Keep each pull request focused on one coherent change. Avoid mixing unrelated
refactoring or formatting with behavior changes.

The pull request description must include:

- What changed and why
- The effect on users or maintainers
- Tests and other checks that were run
- Relevant checks that were not run and why
- Documentation updates for changed behavior or workflows
- A `CHANGELOG.md` update when the change is user-visible

Add or update tests when behavior changes. Ensure documentation and
repository-managed examples remain consistent with the implemented behavior.

## License

By submitting a contribution, you agree that it may be distributed under the
repository's [MIT License](LICENSE).
