use std::{env, error::Error};
use vergen::{BuildBuilder, CargoBuilder, Emitter, RustcBuilder};

fn main() -> Result<(), Box<dyn Error>> {
    // Configure build instructions
    let build = BuildBuilder::default().build_timestamp(true).build()?;

    let cargo = CargoBuilder::default().target_triple(true).features(true).build()?;

    let rustc = RustcBuilder::default().semver(true).commit_hash(true).build()?;

    // Emit instructions
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .emit()?;

    // Check for the Rust compiler commit hash
    if let Ok(rustc_hash) = env::var("VERGEN_RUSTC_COMMIT_HASH") {
        let sha_short = &rustc_hash[..7];

        // Check if the git working directory is dirty
        let is_dirty =
            env::var("VERGEN_GIT_DIRTY").unwrap_or_else(|_| "false".to_string()) == "true";

        // Check if we're not on a tag
        let not_on_tag = env::var("VERGEN_GIT_DESCRIBE")
            .unwrap_or_else(|_| String::new())
            .trim()
            .ends_with(&format!("-g{sha_short}"));

        // Determine if we are in dev mode
        let is_dev = is_dirty || not_on_tag;

        // Set the version suffix
        println!("cargo:rustc-env=MEV_VERSION_SUFFIX={}", if is_dev { "-dev" } else { "" });
    }

    Ok(())
}
