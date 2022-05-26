mod package;
mod remote;

pub const GENERATED_FILE_NAME: &str = "archive.json";

pub use package::{
    GeneratedDefinedPackage, GenericPackage, InstalledPackageContainer, UserDefinedPackage,
};
pub use remote::{
    AuthScript, AuthStrategy, LocalPackageRepository, PackageRepository, RemotePackage,
    S3PackageRepository,
};
