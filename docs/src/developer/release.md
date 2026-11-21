# Releasing TSFFS

1. Run check script: `./scripts/check.sh`
    - This will report issues with formatting (C and Python formatting can be ignored
      for releases, markdown and Rust issues should be fixed)
    - This will perform most checks done in CI including dependencies
    - Any dependencies that are outdated or flag vulnerabilities in audits should be
      updated
    - Any code which has breaking changes (very rare) should be fixed
2. Bump both `version` fields in `Cargo.toml` (`[package]` and
   `[package.metadata.simics]`) and merge the change to `main`
3. Tag the merge commit with the `[package.metadata.simics]` version and push the tag:
   `git tag v6.2.0 && git push origin v6.2.0`
4. The `Build And Test` workflow runs on the tag. Once every job passes, its `Draft
   GitHub Release` job creates a draft release with generated notes and the Linux and
   Windows packages for SIMICS 6 and 7, named
   `simics-pkg-31337-<tsffs version>-<simics base version>-<host>.ispm`
    - The job fails if the tag does not match the `[package.metadata.simics]` version
    - If a job fails for an unrelated reason, re-run the failed jobs. The release job
      reuses the existing release and replaces its packages
5. Review the notes of the draft release on GitHub and publish it
