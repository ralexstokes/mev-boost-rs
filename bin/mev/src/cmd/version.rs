//! Version information for the mev binary.

use std::fmt;

/// Represents the complete version information for the mev binary.
#[derive(Debug)]
pub(crate) struct Version;

impl Version {
    #[must_use]
    pub const fn short_version() -> &'static str {
        concat!("v", env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH", "unknown"), ")")
    }

    #[must_use]
    pub const fn long_version() -> &'static str {
        #[cfg(all(feature = "boost", feature = "build", feature = "relay"))]
        {
            concat!(
                "Version:  ",
                env!("CARGO_PKG_VERSION"),
                "\n",
                "Commit:   ",
                env!("GIT_HASH", "unknown"),
                "\n",
                "Features: boost, build, relay"
            )
        }

        #[cfg(all(feature = "boost", feature = "build", not(feature = "relay")))]
        {
            concat!(
                "Version:  ",
                env!("CARGO_PKG_VERSION"),
                "\n",
                "Commit:   ",
                env!("GIT_HASH", "unknown"),
                "\n",
                "Features: boost, build"
            )
        }

        #[cfg(all(feature = "boost", not(feature = "build"), not(feature = "relay")))]
        {
            concat!(
                "Version:  ",
                env!("CARGO_PKG_VERSION"),
                "\n",
                "Commit:   ",
                env!("GIT_HASH", "unknown"),
                "\n",
                "Features: boost"
            )
        }

        #[cfg(not(any(
            all(feature = "boost", feature = "build", feature = "relay"),
            all(feature = "boost", feature = "build"),
            all(feature = "boost", not(feature = "build"), not(feature = "relay"))
        )))]
        {
            concat!(
                "Version:  ",
                env!("CARGO_PKG_VERSION"),
                "\n",
                "Commit:   ",
                env!("GIT_HASH", "unknown"),
                "\n",
                "Features: none"
            )
        }
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
    fn test_version_display() {
        let version_str = Version::long_version();
        assert!(version_str.contains("Version:"));
        assert!(version_str.contains("Commit:"));
        assert!(version_str.contains("Features:"));
    }

    #[test]
    fn test_short_version() {
        let version_str = Version::short_version();
        assert!(version_str.starts_with("v"));
        assert!(version_str.contains(env!("CARGO_PKG_VERSION")));
        assert!(version_str.contains("("));
        assert!(version_str.contains(")"));
    }
}