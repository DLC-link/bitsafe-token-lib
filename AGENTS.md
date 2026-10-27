# Agent notes for bitsafe-token-lib

## After a release: update the docs of every supported asset

A release of this crate is not finished until the public docs describe it.
Each supported asset has its own docs space at
[docs.bitsafe.finance](https://docs.bitsafe.finance). The supported assets
are the modules in `src/tokens/` (today `cbtc` and `beth`). After you cut a
release, you and the pilot update the docs of every one of them.

The docs live in GitBook, and docs.bitsafe.finance publishes them. Work on
them through the GitBook API. The API needs a GitBook access token, which
the pilot gives you. Keep the token out of the repository, and out of any
file that git tracks.

Make the docs changes through one GitBook change request for each asset. The
pilot reviews the change requests and decides when they merge.

### What to check

- **Version pins.** The `bitsafe-token-lib` tag and package version, the
  `canton-lib` tag, the `canton-api-client` version, and the Canton
  participant version. They appear in prose, tables, `Cargo.toml` snippets
  and `git checkout` commands.
- **Versioned GitHub links.** Every `blob`, `tree` and `releases/tag` link
  into this repository or `canton-lib` moves to the new tag. Confirm that each
  linked path still exists at the new tag, and that each line anchor still
  points at the same code.
- **Code samples and commands.** Module paths, function and field names,
  example names, and environment variables must match the release. Compile
  the samples against the new tag.
- **Breaking changes.** Read the changelogs of this crate and of `canton-lib`.
  Each breaking change needs a row in the migration table on the
  compatibility page. Also correct any page that describes the old behaviour.
- **Compatibility page.** Update the baseline table, the matrix row and its
  "last verified" date. The release notes section links to the GitHub
  releases pages. It does not summarize each release.
- **DAR bundle.** If `dars/` changed, update the DAR names, versions and
  package facts that the pages state.

### How to verify

- Build every example at the new tag.
- Run the read-only examples against devnet with credentials that the pilot
  gives you. Run an example that writes to the ledger only with the pilot's
  approval.
- After the change requests merge, scan all pages of each asset for old
  version strings, not only the pages that you edited. Then open the
  published pages and confirm the links.

### Pitfalls

- GitBook does not resolve relative page links in imported markdown. Convert
  them to page links before you upload a page.
- The docs are public. Do not link private repositories, and do not put
  credentials, tokens or party secrets in a page.
