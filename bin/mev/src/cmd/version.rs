use const_format::{concatcp, str_index};
use std::fmt;

/// The human readable name of the client
pub const NAME_CLIENT: &str = "mev";

/// The latest version from Cargo.toml
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version suffix (-dev or empty)
pub const VERSION_SUFFIX: &str = match option_env!("MEV_VERSION_SUFFIX") {
    Some(suffix) => suffix,
    None => "",
};

/// The build timestamp
pub const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP", "unknown");

/// The target triple
pub const CARGO_TARGET_TRIPLE: &str = env!("VERGEN_CARGO_TARGET_TRIPLE", "unknown");

/// The rustc version
pub const RUSTC_VERSION: &str = env!("VERGEN_RUSTC_SEMVER", "unknown");

/// The full rustc commit hash
pub const GIT_COMMIT_HASH: &str = env!("VERGEN_RUSTC_COMMIT_HASH", "unknown");

/// The short rustc commit hash (first 8 characters)
pub const GIT_COMMIT_SHORT: &str = str_index!(GIT_COMMIT_HASH, ..8);

/// The build features.
pub const VERGEN_CARGO_FEATURES: &str = env!("VERGEN_CARGO_FEATURES");

/// The short version information
pub const SHORT_VERSION: &str =
    concatcp!(NAME_CLIENT, " v", CARGO_PKG_VERSION, VERSION_SUFFIX, " (", GIT_COMMIT_SHORT, ")");

/// The long version information
pub const LONG_VERSION: &str = concatcp!(
    "Version:     ",
    CARGO_PKG_VERSION,
    VERSION_SUFFIX,
    "\n",
    "Commit:      ",
    GIT_COMMIT_HASH,
    "\n",
    "Built:       ",
    BUILD_TIMESTAMP,
    "\n",
    "Target:      ",
    CARGO_TARGET_TRIPLE,
    "\n",
    "Rustc:       ",
    RUSTC_VERSION,
    "\n",
    "Features:    ",
    VERGEN_CARGO_FEATURES
);

#[derive(Debug)]
pub(crate) struct Version;

impl Version {
    #[must_use]
    pub const fn short_version() -> &'static str {
        SHORT_VERSION
    }

    #[must_use]
    pub const fn long_version() -> &'static str {
        LONG_VERSION
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::long_version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_version_information() {
        let long_version_info = Version::long_version();
        assert!(long_version_info.contains("Version:"));
        assert!(long_version_info.contains("Commit:"));
        assert!(long_version_info.contains("Features:"));
        assert!(long_version_info.contains("Built:"));
        assert!(long_version_info.contains("Target:"));
    }

    #[test]
    fn test_short_version_information() {
        let short_version_info = Version::short_version();
        assert!(short_version_info.starts_with(NAME_CLIENT));
        assert!(short_version_info.contains(CARGO_PKG_VERSION));
        assert!(short_version_info.contains("("));
        assert!(short_version_info.contains(")"));
    }
}
