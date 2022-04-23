use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
};

use owo_colors::OwoColorize;
use remove_dir_all::remove_dir_all;
use semver::Version;
use serde::{Deserialize, Serialize};

use super::package::SharedPackageConfig;
use crate::data::{config::Config, package::PackageConfig};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileRepository {
    pub artifacts: HashMap<String, HashMap<Version, SharedPackageConfig>>,
}

impl Default for FileRepository {
    #[inline]
    fn default() -> Self {
        FileRepository {
            artifacts: HashMap::new(),
        }
    }
}

impl FileRepository {
    pub fn get_artifacts_from_id(
        &self,
        id: &String,
    ) -> Option<&HashMap<Version, SharedPackageConfig>> {
        self.artifacts.get(id)
    }

    pub fn get_artifact(&self, id: &String, version: &Version) -> Option<&SharedPackageConfig> {
        match self.artifacts.get(id) {
            Some(artifacts) => artifacts.get(version),
            None => None,
        }
    }

    pub fn add_artifact(
        &mut self,
        package: SharedPackageConfig,
        project_folder: PathBuf,
        binary_path: Option<PathBuf>,
    ) {
        let id_artifacts = match self
            .artifacts
            .try_insert(package.config.info.id.clone(), HashMap::new())
        {
            Ok(e) => e,
            Err(e) => &mut e.value,
        };

        self.add_artifact(package, project_folder, binary_path);
        id_artifacts.insert(package.config.info.version.clone(), package);
    }

    fn add_to_cache(
        package: &SharedPackageConfig,
        project_folder: PathBuf,
        binary_path: Option<PathBuf>,
    ) {
        println!(
            "Adding cache for local dependency {} {}",
            package.config.info.id.bright_red(),
            package.config.info.version.bright_green()
        );
        let config = Config::read_combine();
        let base_path = config
            .cache
            .unwrap()
            .join(&package.config.info.id)
            .join(package.config.info.version.to_string());

        let src_path = base_path.join("src");
        let lib_path = base_path.join("lib");
        let tmp_path = base_path.join("tmp");

        let so_path = lib_path.join(package.config.get_so_name());
        let debug_so_path = lib_path.join(format!("debug_{}", package.config.get_so_name()));

        // Downloads the repo / zip file into src folder w/ subfolder taken into account

        // if the tmp path exists, but src doesn't, that's a failed cache, delete it and try again!
        if tmp_path.exists() {
            remove_dir_all(&tmp_path).expect("Failed to remove existing tmp folder");
        }

        if src_path.exists() {
            remove_dir_all(&src_path).expect("Failed to remove existing src folder");
        }

        std::fs::create_dir_all(&src_path.parent().unwrap()).expect("Failed to create lib path");

        let shared_path = project_folder.join(package.config.shared_dir);
        let original_package_file_path = project_folder.join("qpm.json");

        std::fs::copy(shared_path, src_path.join(package.config.shared_dir)).expect(
            format!(
                "Unable to copy from {:?} to {:?}",
                shared_path,
                src_path.join(package.config.shared_dir)
            )
            .as_str(),
        );
        std::fs::copy(original_package_file_path, src_path.join("qpm.json")).expect(
            format!(
                "Unable to copy from {:?} to {:?}",
                original_package_file_path,
                src_path.join("qpm.json")
            )
            .as_str(),
        );

        if let Some(binary_path_unwrapped) = binary_path {
            std::fs::copy(binary_path_unwrapped, so_path).expect(
                format!(
                    "Unable to copy from {:?} to {:?}",
                    binary_path_unwrapped, so_path
                )
                .as_str(),
            );
        }

        let package_path = src_path.join("qpm.json");
        let downloaded_package = PackageConfig::read_path(package_path);

        // check if downloaded config is the same version as expected, if not, panic
        if downloaded_package.info.version != package.config.info.version {
            panic!(
                "Downloaded package ({}) version ({}) does not match expected version ({})!",
                package.config.info.id.bright_red(),
                downloaded_package.info.version.to_string().bright_green(),
                package.config.info.version.to_string().bright_green(),
            )
        }
    }

    /// always gets the global config
    pub fn read() -> Self {
        let path = Self::global_file_repository_path();
        std::fs::create_dir_all(Self::global_repository_dir())
            .expect("Failed to make config folder");

        if let Ok(mut file) = std::fs::File::open(path) {
            // existed
            let mut config_str = String::new();
            file.read_to_string(&mut config_str)
                .expect("Reading data failed");

            serde_json::from_str::<Self>(&config_str).expect("Deserializing package failed")
        } else {
            // didn't exist
            Self {
                ..Default::default()
            }
        }
    }

    pub fn write(&self) {
        let config = serde_json::to_string_pretty(&self).expect("Serialization failed");
        let path = Self::global_file_repository_path();

        std::fs::create_dir_all(Self::global_repository_dir())
            .expect("Failed to make config folder");
        let mut file = std::fs::File::create(path).expect("create failed");
        file.write_all(config.as_bytes()).expect("write failed");
        println!("Saved Config!");
    }

    pub fn global_file_repository_path() -> PathBuf {
        Self::global_repository_dir().join("qpm.repository.json")
    }

    pub fn global_repository_dir() -> PathBuf {
        dirs::config_dir().unwrap().join("QPM-Rust")
    }
}
