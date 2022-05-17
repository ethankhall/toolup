mod package;
mod remote;

pub const GENERATED_FILE_NAME: &str = "archive.json";

pub use package::{GeneratedDefinedPackage, InstalledPackageContainer, UserDefinedPackage};
pub use remote::{AuthScript, AuthStrategy, PackageRepository, RemotePackage, LocalPackageRepository, S3PackageRepository};
