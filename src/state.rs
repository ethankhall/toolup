use crate::util::{create_link, GlobalFolders};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::id;
use thiserror::Error;
use tracing::field::debug as tracing_wrap;
use tracing::{debug, error};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct GlobalInstalledState {
    #[serde(flatten)]
    pub state: VersionedGlobalState,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "version", rename_all = "kebab-case")]
pub enum VersionedGlobalState {
    #[serde(rename = "v1")]
    V1(v1::InstalledState),
}

#[derive(Error, Debug)]
pub enum StateError {
    #[error("Unable to obtain lock. This means that either another processes is using it, or something bad has happened. Please look at the lock located at {path} to resolve the issue.")]
    UnableToObtainLock { path: String },
    #[error("Attempted to update the global state, but found that the statefile has been updated since this process state. Refusing to update the state file when it may be out-of-date. It was updated at {found:?} where we expected the last update to be {expected:?}.")]
    StateFileOutOfDate {
        expected: Option<DateTime<Utc>>,
        found: Option<DateTime<Utc>>,
    },
    #[error("Package {name}@{version} was not installed.")]
    PackageNotInstalled { name: String, version: String },
    #[error("There was no binary named {name}@{version} installed.")]
    NoSuchBinary { name: String, version: String },
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UknownError(#[from] anyhow::Error),
}

#[derive(Debug, Default)]
pub struct StateContainer {
    pub updated_at: Option<DateTime<Utc>>,
    pub current_state: v1::InstalledState,
}

impl StateContainer {
    pub fn list_installed_packages(&self) -> Vec<PackageDescription> {
        self.current_state
            .installed_packages
            .values()
            .map(|x| self.current_state.describe_package(x))
            .collect()
    }

    pub fn describe_package(&self, name: &str) -> Option<PackageDescription> {
        self.current_state
            .current_packages
            .get(name)
            .map(|x| self.current_state.describe_package(x))
    }

    pub fn remove_packages(&mut self, packages_to_remove: Vec<PackageDescription>) {
        for package in packages_to_remove {
            self.current_state.remove_package_by_id(&package.package_id);
        }
    }
}

pub async fn get_current_state(state_path: &Path) -> Result<StateContainer, StateError> {
    if !state_path.exists() {
        return Ok(Default::default());
    }

    debug!("Reading state from {:?}", state_path);
    let global_state: GlobalInstalledState = serde_json::from_reader(File::open(state_path)?)?;

    debug!(global_state = tracing_wrap(&global_state));

    // future, we woul update state file here.
    let VersionedGlobalState::V1(state) = global_state.state;

    Ok(StateContainer {
        current_state: state,
        updated_at: Some(global_state.updated_at),
    })
}

pub async fn write_state(
    state_path: &Path,
    state_container: StateContainer,
) -> Result<StateContainer, StateError> {
    let parent_path = state_path.parent().expect("There to be a partent");
    if !parent_path.exists() {
        fs::create_dir_all(parent_path)?;
    }

    let pid = id();
    let now = chrono::Utc::now();

    let state_file_name = state_path
        .file_name()
        .expect("The state file to have a valid filename")
        .to_os_string()
        .into_string()
        .expect("State file to have a valid filename.");
    let lock_file_path = parent_path.join(format!("{}.lock", state_file_name));

    debug!("Trying to get lock on {:?}", &lock_file_path);
    let mut lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_file_path)?;
    let mut has_lock = false;
    for _i in 0..10 {
        match lock_file.try_lock_exclusive() {
            Ok(_) => {
                has_lock = true;
                break;
            }
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
    if !has_lock {
        return Err(StateError::UnableToObtainLock {
            path: lock_file_path.display().to_string(),
        });
    }

    debug!("Obtained lock on {:?}", &lock_file_path);

    lock_file.set_len(0)?; // truncate the file
    let lock_write_1 = writeln!(lock_file, "{}", pid);
    let lock_write_2 = writeln!(lock_file, "This lock file was created by PID {} at {}. If you see this file after {} please delete it, something terrible went wrong.", pid, now, now);

    match (lock_write_1, lock_write_2) {
        (Err(e), _) | (Ok(_), Err(e)) => {
            error!(target: "user", "Unable to write to lock, ignoring error.");
            error!(
                "Unable to write to lock. Since we have the lock it should be safe to ingore. {:?}",
                e
            );
        }
        _ => {}
    }

    let result = with_write_lock(state_path, state_container).await;
    fs::remove_file(&lock_file_path)?;
    lock_file.unlock()?;

    debug!(
        "Deleted lock file {:?} and released the lock",
        lock_file_path
    );

    result
}

async fn with_write_lock(
    state_path: &Path,
    state_container: StateContainer,
) -> Result<StateContainer, StateError> {
    let current_sate = get_current_state(state_path).await?;
    if state_container.updated_at != current_sate.updated_at {
        return Err(StateError::StateFileOutOfDate {
            expected: state_container.updated_at,
            found: current_sate.updated_at,
        });
    }

    let new_state = GlobalInstalledState {
        updated_at: Utc::now(),
        state: VersionedGlobalState::V1(state_container.current_state),
    };

    debug!("Will write {:?} to {:?}", new_state, state_path);
    let state_contents = serde_json::to_string_pretty(&new_state)?;
    fs::write(state_path, state_contents)?;

    debug!("State file {:?} written successfully", state_path);

    get_current_state(state_path).await
}

pub async fn update_links(
    state_container: &StateContainer,
    global_folder: &GlobalFolders,
) -> Result<(), StateError> {
    let current_state = &state_container.current_state;
    let link_dir = global_folder.get_link_dir();
    let mut installed_tools: HashSet<String> = Default::default();

    if !link_dir.exists() {
        fs::create_dir_all(&link_dir)?;
    }

    for entry in fs::read_dir(&link_dir)? {
        let entry = entry?;
        let filename = entry
            .path()
            .file_name()
            .expect("The state file to have a valid filename")
            .to_os_string()
            .into_string()
            .expect("State file to have a valid filename.");

        installed_tools.insert(filename);
    }

    for name in current_state.current_binaries.keys() {
        let mut current_exec = std::env::current_exe()?;
        current_exec.pop();
        current_exec.push("toolup-shim");

        let binary_link = Path::join(&link_dir, name);
        debug!("Setting up link {} to {:?}", name, binary_link);
        if binary_link.exists() || binary_link.is_symlink() {
            debug!("Removing existing binary link");
            std::fs::remove_file(&binary_link)?;
        }
        create_link(current_exec, binary_link)?;
        installed_tools.remove(name);
    }

    for missing_binary in installed_tools {
        let binary_link = Path::join(&link_dir, missing_binary);
        if binary_link.exists() {
            debug!("Removing {:?}", binary_link);
            std::fs::remove_file(&binary_link)?;
        }
    }

    debug!("Link updates complete");

    Ok(())
}

#[derive(Debug)]
pub struct PackageDescription {
    pub name: String,
    pub version: String,
    pub binaries: BTreeMap<String, bool>,
    pub remote_name: Option<String>,
    pub etag: Option<String>,
    pub package_id: String,
}

mod v1 {
    use super::StateError;
    use crate::model::{GenericPackage, InstalledPackageContainer};
    use derivative::Derivative;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use std::hash::{Hash, Hasher};
    use std::path::Path;
    use tracing::{debug, warn};

    #[derive(Serialize, Deserialize, Derivative, Debug, Clone)]
    #[serde(rename_all = "kebab-case")]
    #[derivative(Eq, PartialOrd, Ord, PartialEq)]
    pub struct InstalledPackage {
        pub id: String,
        pub name: String,
        pub version: String,
        #[derivative(PartialEq = "ignore")]
        pub package_dir: String,
        pub remote_name: Option<String>,
        pub etag: Option<String>,
    }

    impl GenericPackage for &InstalledPackage {
        fn name(&self) -> String {
            self.name.clone()
        }

        fn version(&self) -> String {
            self.version.clone()
        }

        fn id(&self) -> String {
            self.id.clone()
        }
    }

    impl Hash for InstalledPackage {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.name.hash(state);
            self.version.hash(state);
        }
    }

    impl From<&InstalledPackageContainer> for InstalledPackage {
        fn from(container: &InstalledPackageContainer) -> Self {
            Self {
                id: crate::util::make_package_id(
                    &container.package.name,
                    &container.package.version,
                ),
                name: container.package.name.clone(),
                version: container.package.version.clone(),
                package_dir: container.path_to_root.clone(),
                remote_name: container.remote_name.clone(),
                etag: container.etag.clone(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Derivative)]
    #[derivative(Eq, PartialOrd, Ord, PartialEq)]
    #[serde(rename_all = "kebab-case")]
    pub struct InstalledBinary {
        pub id: String,
        pub name: String,
        pub version: String,
        #[derivative(PartialEq = "ignore")]
        pub path_to_exec: String,
        pub package_id: String,
    }

    impl InstalledBinary {
        fn new<P>(package: &P, binary_name: String, path: String) -> Self
        where
            P: GenericPackage,
        {
            InstalledBinary {
                id: format!(
                    "urn:package:toolup/{}/{}/{}",
                    package.name(),
                    package.version(),
                    binary_name,
                ),
                name: binary_name,
                version: package.version(),
                path_to_exec: path,
                package_id: package.id(),
            }
        }
    }

    impl Hash for InstalledBinary {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.name.hash(state);
            self.version.hash(state);
            self.package_id.hash(state);
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    #[serde(rename_all = "kebab-case")]
    pub struct InstalledState {
        pub installed_packages: BTreeMap<String, InstalledPackage>,
        pub installed_binaries: BTreeMap<String, InstalledBinary>,
        pub current_binaries: BTreeMap<String, InstalledBinary>,
        #[serde(default = "Default::default")]
        pub current_packages: BTreeMap<String, InstalledPackage>,
    }

    impl InstalledState {
        pub fn describe_package(&self, package: &InstalledPackage) -> super::PackageDescription {
            let mut binaries_installed = BTreeMap::new();
            for binary in self.installed_binaries.values() {
                if binary.package_id == package.id {
                    binaries_installed.insert(binary.name.clone(), false);
                }
            }

            for (name, binary) in &self.current_binaries {
                if binary.package_id == package.id {
                    binaries_installed
                        .entry(name.to_string())
                        .and_modify(|x| *x = true);
                }
            }

            super::PackageDescription {
                name: package.name.clone(),
                version: package.version.clone(),
                binaries: binaries_installed,
                remote_name: package.remote_name.clone(),
                package_id: package.id.clone(),
                etag: package.etag.clone(),
            }
        }

        pub fn remove_package_by_id(&mut self, id: &str) {
            let mut binary_ids_to_remove = Vec::new();
            for (binary_id, binary) in &self.installed_binaries {
                if binary.package_id == id {
                    binary_ids_to_remove.push(binary_id.to_owned())
                }
            }

            for binary_id in binary_ids_to_remove {
                self.installed_binaries.remove(&binary_id);
            }

            let mut binary_name_to_remove: Vec<String> = Vec::new();
            for (name, binary) in &self.current_binaries {
                if binary.package_id == id {
                    binary_name_to_remove.push(name.to_owned());
                }
            }

            for binary_name in binary_name_to_remove {
                self.current_binaries.remove(&binary_name);
            }

            self.installed_packages.remove(id);
        }

        pub fn remove_packages(&mut self, packages_to_remove: Vec<InstalledPackage>) {
            for package in packages_to_remove {
                self.remove_package_by_id(&package.id);
            }
        }

        pub fn add_installed_package(&mut self, container: &InstalledPackageContainer) {
            let package = InstalledPackage::from(container);
            debug!("Adding {:?}.", &package);

            self.installed_packages.insert(package.id.clone(), package);
            for (binary_name, relative_path) in &container.package.entrypoints {
                let binary_path = Path::new(&container.path_to_root)
                    .join(relative_path)
                    .display()
                    .to_string();
                let binary =
                    InstalledBinary::new(&container.package, binary_name.to_string(), binary_path);

                debug!("Adding binary {:?}", binary);

                self.installed_binaries.insert(binary.id.clone(), binary);
            }
        }

        pub fn make_package_current(
            &mut self,
            container: &InstalledPackageContainer,
        ) -> Result<(), StateError> {
            let package = InstalledPackage::from(container);

            debug!("Setting {:?} to be current.", &package);

            if !self.installed_packages.contains_key(&package.id) {
                return Err(StateError::PackageNotInstalled {
                    name: package.name,
                    version: package.version,
                });
            }

            let package_to_remove = self.current_packages.get(&package.name);

            if let Some(package_to_remove) = package_to_remove {
                let mut binary_names_to_remove = Vec::new();
                for (name, binary) in &self.current_binaries {
                    if binary.package_id == package_to_remove.id {
                        binary_names_to_remove.push(name.clone());
                    }
                }

                for binary in binary_names_to_remove {
                    self.current_binaries.remove(&binary);
                }
            }

            for binary in self.installed_binaries.values() {
                if binary.package_id == package.id {
                    let existing = self
                        .current_binaries
                        .insert(binary.name.clone(), binary.clone());
                    if let Some(old_binary) = existing {
                        if old_binary.package_id != package.id {
                            warn!(target: "user", "{} is replacing a managed binary", package.name);
                        }
                        debug!("Removing old binary {:?}.", old_binary);
                    }

                    debug!("Setting binary {:?} to current.", binary);
                }
            }

            if let Some(existing) = &self
                .current_packages
                .insert(package.name.clone(), package.clone())
            {
                debug!("Replacing existing packag{:?} with {:?}", existing, package);
            }

            Ok(())
        }

        pub fn get_current_binary_path(&self, name: &str) -> Result<String, StateError> {
            let binary = match self.current_binaries.get(name) {
                None => {
                    return Err(StateError::NoSuchBinary {
                        name: name.to_string(),
                        version: "CURRENT".to_string(),
                    })
                }
                Some(binary) => binary,
            };

            Ok(binary.path_to_exec.clone())
        }

        pub fn get_binary_path(&self, name: &str, version: &str) -> Result<String, StateError> {
            for binary in self.installed_binaries.values() {
                if binary.name == name && binary.version == version {
                    return Ok(binary.path_to_exec.clone());
                }
            }
            Err(StateError::NoSuchBinary {
                name: name.to_string(),
                version: version.to_string(),
            })
        }
    }

    #[test]
    fn add_package_one() {
        let mut installed_state = InstalledState::default();
        let container = make_stub_package_container("foo", "1.2.3", 1);
        installed_state.add_installed_package(&container);

        assert_eq!(installed_state.current_binaries.len(), 0);
        assert_eq!(installed_state.installed_binaries.len(), 1);
        assert_eq!(installed_state.installed_packages.len(), 1);

        installed_state.make_package_current(&container).unwrap();
        assert_eq!(installed_state.current_binaries.len(), 1);

        let bin = installed_state.current_binaries.get("bin-1").unwrap();
        assert_eq!(bin.name, "bin-1");
        assert_eq!(bin.version, "1.2.3");
        assert_eq!(bin.path_to_exec, "/tmp/fake/bin-1");
    }

    #[test]
    fn handle_paths_moving() {
        let mut installed_state = InstalledState::default();
        let container = make_stub_package_container("foo", "1.2.3", 1);
        installed_state.add_installed_package(&container);
        installed_state.make_package_current(&container).unwrap();

        // Install the package again, at a different path.
        let mut container = make_stub_package_container("foo", "1.2.3", 1);
        container.path_to_root = "/tmp/foo/fake".to_owned();
        installed_state.add_installed_package(&container);
        installed_state.make_package_current(&container).unwrap();

        assert_eq!(installed_state.current_binaries.len(), 1);
        assert_eq!(installed_state.installed_binaries.len(), 1);
        assert_eq!(installed_state.installed_packages.len(), 1);

        let bin = installed_state.current_binaries.get("bin-1").unwrap();
        assert_eq!(bin.name, "bin-1");
        assert_eq!(bin.version, "1.2.3");
        assert_eq!(bin.path_to_exec, "/tmp/foo/fake/bin-1");
    }

    #[test]
    fn add_overlapping_packages() {
        let mut installed_state = InstalledState::default();
        let container1 = make_stub_package_container("foo", "1.2.3", 1);
        installed_state.add_installed_package(&container1);

        let container2 = make_stub_package_container("foo", "2.3.4", 3);
        installed_state.add_installed_package(&container2);

        assert_eq!(installed_state.current_binaries.len(), 0);
        assert_eq!(installed_state.installed_binaries.len(), 4);
        assert_eq!(installed_state.installed_packages.len(), 2);

        // install version
        {
            installed_state.make_package_current(&container1).unwrap();
            let installed_package = InstalledPackage::from(&container1);
            assert_eq!(installed_state.current_binaries.len(), 1);

            let bin = installed_state.current_binaries.get("bin-1").unwrap();
            assert_eq!(
                bin,
                &fake_binary("bin-1", "1.2.3", false, &installed_package)
            );
        }

        // install new version
        {
            installed_state.make_package_current(&container2).unwrap();
            let installed_package = InstalledPackage::from(&container2);
            assert_eq!(installed_state.current_binaries.len(), 3);

            let bin = installed_state.current_binaries.get("bin-1").unwrap();
            assert_eq!(
                bin,
                &fake_binary("bin-1", "2.3.4", false, &installed_package)
            );

            let bin = installed_state.current_binaries.get("bin-2").unwrap();
            assert_eq!(
                bin,
                &fake_binary("bin-2", "2.3.4", false, &installed_package)
            );

            let bin = installed_state.current_binaries.get("bin-3").unwrap();
            assert_eq!(
                bin,
                &fake_binary("bin-3", "2.3.4", true, &installed_package)
            );
        }

        // Roll back to old version, should remove all binaries of package
        {
            installed_state.make_package_current(&container1).unwrap();
            assert_eq!(installed_state.current_binaries.len(), 1);

            let installed_package = InstalledPackage::from(&container1);
            let bin = installed_state.current_binaries.get("bin-1").unwrap();
            assert_eq!(
                bin,
                &fake_binary("bin-1", "1.2.3", false, &installed_package)
            );
        }
    }

    #[cfg(test)]
    fn fake_binary(
        name: &str,
        _version: &str,
        is_sub: bool,
        package: &InstalledPackage,
    ) -> InstalledBinary {
        let path = if is_sub {
            format!("/tmp/fake/sub/{}", name)
        } else {
            format!("/tmp/fake/{}", name)
        };
        InstalledBinary::new(&package, name.to_string(), path)
    }

    #[test]
    fn will_fail_when_package_not_installed() {
        let mut installed_state = InstalledState::default();
        let container = make_stub_package_container("foo", "1.2.3", 1);

        assert_eq!(installed_state.current_binaries.len(), 0);
        assert_eq!(installed_state.installed_binaries.len(), 0);
        assert_eq!(installed_state.installed_packages.len(), 0);

        let error = installed_state
            .make_package_current(&container)
            .unwrap_err();
        assert_eq!(error.to_string(), "Package foo@1.2.3 was not installed.");
    }

    #[test]
    fn package_remove_is_complete() {
        let mut installed_state = InstalledState::default();
        let container = make_stub_package_container("foo", "1.2.3", 1);
        installed_state.add_installed_package(&container);

        assert_eq!(installed_state.current_binaries.len(), 0);
        assert_eq!(installed_state.installed_binaries.len(), 1);
        assert_eq!(installed_state.installed_packages.len(), 1);

        installed_state.make_package_current(&container).unwrap();
        assert_eq!(installed_state.current_binaries.len(), 1);

        let installed_package = installed_state
            .installed_packages
            .values()
            .next()
            .unwrap()
            .clone();

        installed_state.remove_package_by_id(&installed_package.id);
        assert_eq!(installed_state.current_binaries.len(), 0);
        assert_eq!(installed_state.installed_binaries.len(), 0);
        assert_eq!(installed_state.installed_packages.len(), 0);
    }

    #[cfg(test)]
    fn make_stub_package_container(
        package_name: &str,
        version: &str,
        number_of_binaries: u32,
    ) -> InstalledPackageContainer {
        use crate::model::GeneratedDefinedPackage;

        let mut entrypoints = BTreeMap::new();
        for i in 1..(number_of_binaries + 1) {
            let path = if i % 3 == 0 {
                format!("sub/bin-{}", i)
            } else {
                format!("bin-{}", i)
            };
            entrypoints.insert(format!("bin-{}", i), path);
        }

        let package = GeneratedDefinedPackage {
            name: package_name.to_string(),
            entrypoints,
            version: version.to_string(),
            achived_at: chrono::Utc::now(),
            file_hashes: Default::default(),
        };

        InstalledPackageContainer {
            path_to_root: "/tmp/fake".to_string(),
            package,
            remote_name: None,
            etag: None,
        }
    }
}
