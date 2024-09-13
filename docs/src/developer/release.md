# Releasing TSFFS

1. Run check script: `./check.sh`
    - This will report issues with formatting (C and Python formatting can be ignored
      for releases, markdown and Rust issues should be fixed)
    - This will perform most checks done in CI including dependencies
    - Any dependencies that are outdated or flag vulnerabilities in audits should be
      updated
    - Any code which has breaking changes (very rare) should be fixed
