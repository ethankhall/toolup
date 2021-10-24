use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::id;
use thiserror::Error;
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

#[derive(Debug)]
pub struct StateContainer {
    pub updated_at: Option<DateTime<Utc>>,
    pub current_state: v1::InstalledState,
}

impl Default for StateContainer {
    fn default() -> Self {
        Self {
            updated_at: None,
            current_state: Default::default(),
        }
    }
}

pub async fn get_current_state(state_path: &Path) -> Result<StateContainer, StateError> {
    if !state_path.exists() {
        return Ok(Default::default());
    }

    debug!("Reading state from {:?}", state_path);
    let global_state: GlobalInstalledState = serde_json::from_reader(File::open(state_path)?)?;

    debug!("Parsed state is {:?}", global_state);

    // future, we woul update state file here.
    let state = match global_state.state {
        VersionedGlobalState::V1(state) => state,
    };

    Ok(StateContainer {
        current_state: state,
        updated_at: Some(global_state.updated_at.clone()),
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
    let lock_write_1 = write!(lock_file, "{}\n", pid);
    let lock_write_2 = write!(lock_file, "This lock file was created by PID {} at {}. If you see this file after {} please delete it, something terrible went wrong.\n", pid, now, now);

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

pub struct PackageDescription {
    pub name: String,
    pub version: String,
    pub binaries: BTreeMap<String, bool>,
}

mod v1 {
    use super::StateError;
    use crate::model::InstalledPackageContainer;
    use derivative::Derivative;
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, HashSet};
    use std::hash::{Hash, Hasher};
    use std::path::Path;
    use tracing::{debug, warn};

    #[derive(Serialize, Deserialize, Derivative, Debug, Clone)]
    #[serde(rename_all = "kebab-case")]
    #[derivative(Eq, PartialOrd, Ord, PartialEq)]
    pub struct InstalledPackage {
        pub name: String,
        pub version: String,
        #[derivative(PartialEq = "ignore")]
        pub package_dir: String,
        pub remote_name: Option<String>,
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
                name: container.package.name.clone(),
                version: container.package.version.clone(),
                package_dir: container.path_to_root.clone(),
                remote_name: container.remote_name.clone(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Derivative)]
    #[derivative(Eq, PartialOrd, Ord, PartialEq)]
    #[serde(rename_all = "kebab-case")]
    pub struct InstalledBinary {
        pub name: String,
        pub version: String,
        #[derivative(PartialEq = "ignore")]
        pub path_to_exec: String,
        pub package: InstalledPackage,
    }

    impl Hash for InstalledBinary {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.name.hash(state);
            self.version.hash(state);
            self.package.hash(state);
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "kebab-case")]
    pub struct InstalledState {
        pub installed_packages: HashSet<InstalledPackage>,
        pub installed_binaries: HashSet<InstalledBinary>,
        pub current_binaries: BTreeMap<String, InstalledBinary>,
    }

    impl InstalledState {
        pub fn describe_package(&self, package: &InstalledPackage) -> super::PackageDescription {
            let mut binaries_installed = BTreeMap::new();
            for binary in &self.installed_binaries {
                if &binary.package == package {
                    binaries_installed.insert(binary.name.clone(), false);
                }
            }

            for (name, binary) in &self.current_binaries {
                if &binary.package == package {
                    binaries_installed
                        .entry(name.to_string())
                        .and_modify(|x| *x = true);
                }
            }

            super::PackageDescription {
                name: package.name.clone(),
                version: package.version.clone(),
                binaries: binaries_installed,
            }
        }

        pub fn remove_packages(&mut self, packages_to_remove: Vec<InstalledPackage>) {
            for package in packages_to_remove {
                self.remove_package(&package);
            }
        }

        pub fn remove_package(&mut self, package: &InstalledPackage) {
            let mut binaries_to_remove = Vec::new();
            for binary in &self.installed_binaries {
                if &binary.package == package {
                    binaries_to_remove.push(binary.clone())
                }
            }

            for binary in binaries_to_remove {
                self.installed_binaries.remove(&binary);
            }

            let mut binaries_to_remove = Vec::new();
            for (name, binary) in &self.current_binaries {
                if binary.package.name == package.name {
                    binaries_to_remove.push(name.clone());
                }
            }

            for binary in binaries_to_remove {
                self.current_binaries.remove(&binary);
            }

            self.installed_packages.remove(package);
        }

        pub fn add_installed_package(&mut self, container: &InstalledPackageContainer) {
            let package = InstalledPackage::from(container);
            debug!("Adding {:?}.", &package);

            self.installed_packages.replace(package.clone());
            for (binary_name, relative_path) in &container.package.entrypoints {
                let binary = InstalledBinary {
                    name: binary_name.to_string(),
                    version: package.version.clone(),
                    path_to_exec: Path::new(&container.path_to_root)
                        .join(relative_path)
                        .display()
                        .to_string(),
                    package: package.clone(),
                };

                debug!("Adding binary {:?}", binary);

                self.installed_binaries.replace(binary);
            }
        }

        pub fn make_package_current(
            &mut self,
            container: &InstalledPackageContainer,
        ) -> Result<(), StateError> {
            let package = InstalledPackage::from(container);

            debug!("Setting {:?} to be current.", &package);

            if !self.installed_packages.contains(&package) {
                return Err(StateError::PackageNotInstalled {
                    name: package.name,
                    version: package.version,
                });
            }

            let mut binaries_to_remove = Vec::new();
            for (name, binary) in &self.current_binaries {
                if binary.package.name == package.name {
                    binaries_to_remove.push(name.clone());
                }
            }

            for binary in binaries_to_remove {
                self.current_binaries.remove(&binary);
            }

            for binary in &self.installed_binaries {
                if &binary.package == &package {
                    let existing = self
                        .current_binaries
                        .insert(binary.name.clone(), binary.clone());
                    if let Some(old_binary) = existing {
                        if old_binary.package.name != package.name {
                            warn!(target: "user", "{} is replacing a binary that was managed by {}", package.name, old_binary.package.name);
                        }
                        debug!("Removing old binary {:?}.", old_binary);
                    }

                    debug!("Setting binary {:?} to current.", binary);
                }
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
            for binary in &self.installed_binaries {
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

    impl Default for InstalledState {
        fn default() -> Self {
            Self {
                installed_binaries: Default::default(),
                installed_packages: Default::default(),
                current_binaries: Default::default(),
            }
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
        assert_eq!(bin.package, InstalledPackage::from(&container));
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
        assert_eq!(bin.package, InstalledPackage::from(&container));
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
            let installed_package = InstalledPackage::from(&container1);
            assert_eq!(installed_state.current_binaries.len(), 1);

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
        version: &str,
        is_sub: bool,
        package: &InstalledPackage,
    ) -> InstalledBinary {
        let path = if is_sub {
            format!("/tmp/fake/sub/{}", name)
        } else {
            format!("/tmp/fake/{}", name)
        };
        InstalledBinary {
            name: name.to_string(),
            version: version.to_string(),
            path_to_exec: path,
            package: package.clone(),
        }
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
            .iter()
            .next()
            .unwrap()
            .clone();

        installed_state.remove_package(&installed_package);
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
        }
    }
}
