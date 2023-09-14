# Developer Guide

There are some general guidelines and practices built in to this repo for developers
convenience and as requirements for PR.

- [Developer Guide](#developer-guide)
  - [Scripts](#scripts)

## Scripts

There are several scripts for developers:

- [./scripts/fmt.sh](../scripts/fmt.sh): Auto-format files. Run this before committing
  or submitting a PR.
- [./scripts/check.sh](../scripts/check.sh): Check file formatting. Run this before
  committing or submitting a PR.
- [./scripts/check.sh](../scripts/ci.sh): Run CI scripts. It's recommended to run this
  before committing or PR-ing if you're able to save bumper-cars programming.
- [./scripts/cov.sh](../scripts/cov.sh): Generate a coverage report from tests.
- [./scripts/dependabot.sh](../scripts/dependabot.sh): Generate an up to date dependabot
  report. The newest dependabot report must be 2 weeks old or newer for CI to pass.