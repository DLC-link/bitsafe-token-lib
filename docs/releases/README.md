# Create a GitHub release

The release PR contains the package version, lockfile version, README pin, dated changelog entry, and release notes in this directory.
The crate uses Git dependencies and has `publish = false`.

The maintainer reviews and merges the release PR before creating its tag.
The maintainer checks the merged commit and its CI results.
For 0.5.0, the maintainer accepts the Cargo Deny warning tracked in [issue #20](https://github.com/DLC-link/bitsafe-token-lib/issues/20).

The following commands create the 0.5.0 tag and GitHub release after the merge.
Replace `MERGED_COMMIT_SHA` with the full reviewed commit SHA from main.

```sh
git fetch origin main --tags
release_commit=MERGED_COMMIT_SHA
git merge-base --is-ancestor "$release_commit" origin/main
git show "$release_commit:Cargo.toml"
git show "$release_commit:CHANGELOG.md"
git tag -s v0.5.0 "$release_commit" -m "bitsafe-token 0.5.0"
git verify-tag v0.5.0
git push origin refs/tags/v0.5.0
env -u GH_TOKEN -u GITHUB_TOKEN gh release create v0.5.0 \
  --verify-tag --title "bitsafe-token 0.5.0" \
  --notes-file docs/releases/v0.5.0.md
```

The maintainer stops if commit or tag signing fails.
The maintainer never disables signing to complete a release.

The release also requires the external documentation steps in [AGENTS.md](../../AGENTS.md).
Prepare one GitBook change request for each supported asset: CBTC, BETH, and CHYPE.
Update version pins, compatibility tables, migration guidance, and versioned links for the release.
Publish those updates through review and verify the public pages after their change requests merge.
