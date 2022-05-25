pub fn print_version() {
    println!("Build Timestamp: {}", env!("VERGEN_BUILD_TIMESTAMP"));
    println!("Build Version: {}", env!("VERGEN_GIT_SEMVER"));
    println!("Commit SHA: {}", env!("VERGEN_GIT_SHA"));
    println!("Commit Date: {}", env!("VERGEN_GIT_COMMIT_TIMESTAMP"));
    println!("Commit Branch: {}", env!("VERGEN_GIT_BRANCH"));
}
