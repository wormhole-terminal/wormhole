<!-- What does this change and why? Link the issue it fixes (Fixes #123). -->

## Checks

- [ ] `scripts/check.sh` passes (fmt, clippy, tests, the build without devctl)
- [ ] For a UI change: before/after screenshots, light and dark
- [ ] For a change to rendering or the PTY: `scripts/perf.sh` stays within the baseline
- [ ] `CHANGELOG.md` has an entry under Unreleased (or the PR is labelled `no changelog`)
- [ ] The PR title follows the commit rules in CONTRIBUTING.md (`fix(git): …`); a squash merge uses it as the commit's header
