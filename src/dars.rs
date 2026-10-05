//! The DAR check for one asset. The DAR files live in this repository's
//! `dars/` folder, and each asset's `INFO.dar_dirs` names the folders it
//! needs.

use std::path::Path;

use token::dar_check::{self, DarCheckResult};

/// Checks that the participant at `ledger_host` holds the newest package of
/// every DAR in `dar_dirs`, such as `cbtc::INFO.dar_dirs`.
///
/// Each entry of `dar_dirs` is relative to `root`, the folder that holds this
/// repository's `dars/`. A caller in this repository passes
/// `env!("CARGO_MANIFEST_DIR")`. A caller outside it passes a folder that
/// holds a copy of this repository's `dars/`, such as a clone at the same
/// tag.
///
/// # Errors
///
/// Fails when a DAR folder does not exist under `root`, or when the
/// participant does not answer the package list call. A missing package is
/// not an error: the result's `status` is `Fail` and `missing` names it.
pub async fn check_dars(
    root: &Path,
    dar_dirs: &[&str],
    ledger_host: String,
    access_token: String,
) -> Result<DarCheckResult, String> {
    dar_check::check(dar_check::Params {
        ledger_host,
        access_token,
        dar_dirs: dirs_under(root, dar_dirs),
    })
    .await
}

fn dirs_under(root: &Path, dar_dirs: &[&str]) -> Vec<String> {
    dar_dirs
        .iter()
        .map(|dir| root.join(dir).to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{beth, cbtc};

    const ROOT: &str = env!("CARGO_MANIFEST_DIR");

    /// The newest package of each name that the check for `dar_dirs` expects.
    fn expected(dar_dirs: &[&str]) -> Vec<(String, String, String)> {
        dar_check::scan_dar_dirs(&dirs_under(Path::new(ROOT), dar_dirs))
            .expect("the shipped DAR folders")
            .into_iter()
            .map(|info| (info.name, info.version, info.package_id))
            .collect()
    }

    fn version_of<'a>(expected: &'a [(String, String, String)], name: &str) -> Option<&'a str> {
        expected
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, version, _)| version.as_str())
    }

    #[tokio::test]
    async fn check_dars_fails_on_a_root_without_the_dar_folders() {
        let err = check_dars(
            Path::new("/nonexistent"),
            cbtc::INFO.dar_dirs,
            "http://127.0.0.1:9".to_string(),
            "unused".to_string(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            err,
            "DAR directory not found: /nonexistent/dars/dependencies"
        );
    }

    #[test]
    fn the_cbtc_check_expects_cbtc_1_2_1_and_no_beth() {
        let expected = expected(cbtc::INFO.dar_dirs);
        assert!(expected.contains(&(
            "cbtc".to_string(),
            "1.2.1".to_string(),
            "3484f3214004982cc14315f7835d8722af9b141aebfc3ddcc605b2bee338f1bd".to_string(),
        )));
        assert_eq!(version_of(&expected, "beth"), None);
    }

    #[test]
    fn the_beth_check_expects_beth_0_2_0_and_no_cbtc() {
        let expected = expected(beth::INFO.dar_dirs);
        assert!(expected.contains(&(
            "beth".to_string(),
            "0.2.0".to_string(),
            "99a39f943ce8fcb39159f8ed4fabab8c71da14db932347cddcf88bf0ac6e521f".to_string(),
        )));
        assert_eq!(version_of(&expected, "cbtc"), None);
    }

    #[test]
    fn both_checks_expect_the_newest_shared_dependencies() {
        for dar_dirs in [cbtc::INFO.dar_dirs, beth::INFO.dar_dirs] {
            let expected = expected(dar_dirs);
            assert_eq!(version_of(&expected, "splice-util"), Some("0.1.4"));
            assert_eq!(
                version_of(&expected, "utility-credential-v0"),
                Some("0.1.1")
            );
            assert_eq!(
                version_of(&expected, "utility-registry-app-v0"),
                Some("0.8.2")
            );
        }
    }
}
