use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::ModuleId;
use cairo_lang_filesystem::db::{
    CORELIB_CRATE_NAME, CrateConfiguration, CrateIdentifier, CrateSettings, FilesGroup,
    dev_corelib_crate_settings,
};
use cairo_lang_filesystem::ids::{
    CrateId, CrateInput, CrateLongId, Directory, FileId, FileKind, FileLongId, SmolStrId,
    VirtualFile,
};
use cairo_lang_filesystem::{override_file_content, set_crate_config};
pub use cairo_lang_project::*;
use cairo_lang_utils::Intern;
use salsa::Database;

#[derive(thiserror::Error, Debug)]
pub enum ProjectError {
    #[error("Only files with a .cairo extension can be compiled.")]
    BadFileExtension,
    #[error("Couldn't read {path}: No such file.")]
    NoSuchFile { path: String },
    #[error("Couldn't handle {path}: Not a legal path.")]
    BadPath { path: String },
    #[error("Failed to load project config: {0}")]
    LoadProjectError(DeserializationError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InMemoryProject {
    pub main_crate_name: String,
    pub main_crate_files: BTreeMap<String, String>,
    pub corelib_files: BTreeMap<String, String>,
    pub main_crate_settings: Option<CrateSettings>,
}

#[derive(thiserror::Error, Debug)]
pub enum InMemoryProjectError {
    #[error("Main crate name cannot be empty.")]
    EmptyMainCrateName,
    #[error("Missing required file `{path}` in `{crate_name}` crate.")]
    MissingRequiredFile { crate_name: &'static str, path: String },
    #[error("Invalid virtual path `{path}` in `{crate_name}` crate.")]
    InvalidVirtualPath { crate_name: &'static str, path: String },
}

/// Sets up the DB to compile the file at the given path.
/// Returns the input identifier of the generated crate.
pub fn setup_single_file_project(
    db: &mut dyn Database,
    path: &Path,
) -> Result<CrateInput, ProjectError> {
    match path.extension().and_then(OsStr::to_str) {
        Some("cairo") => (),
        _ => {
            return Err(ProjectError::BadFileExtension);
        }
    }
    if !path.exists() {
        return Err(ProjectError::NoSuchFile { path: path.to_string_lossy().to_string() });
    }
    let bad_path_err = || ProjectError::BadPath { path: path.to_string_lossy().to_string() };
    let canonical = path.canonicalize().map_err(|_| bad_path_err())?;
    let file_dir = canonical.parent().ok_or_else(bad_path_err)?;
    let file_stem = path.file_stem().and_then(OsStr::to_str).ok_or_else(bad_path_err)?;
    if file_stem == "lib" {
        let crate_name = file_dir.to_str().ok_or_else(bad_path_err)?;
        let crate_id = CrateId::plain(db, SmolStrId::from(db, crate_name));
        set_crate_config!(
            db,
            crate_id,
            Some(CrateConfiguration::default_for_root(Directory::Real(file_dir.to_path_buf())))
        );
        let crate_id = CrateId::plain(db, SmolStrId::from(db, crate_name));
        Ok(crate_id.long(db).clone().into_crate_input(db))
    } else {
        // If file_stem is not lib, create a fake lib file.
        let crate_id = CrateId::plain(db, SmolStrId::from(db, file_stem));
        set_crate_config!(
            db,
            crate_id,
            Some(CrateConfiguration::default_for_root(Directory::Real(file_dir.to_path_buf())))
        );
        let crate_id = CrateId::plain(db, SmolStrId::from(db, file_stem));
        let module_id = ModuleId::CrateRoot(crate_id);
        let file_id = db.module_main_file(module_id).unwrap();
        override_file_content!(db, file_id, Some(format!("mod {file_stem};").into()));
        let crate_id = CrateId::plain(db, SmolStrId::from(db, file_stem));
        Ok(crate_id.long(db).clone().into_crate_input(db))
    }
}

/// Updates the crate roots from a `ProjectConfig` object.
pub fn update_crate_roots_from_project_config(db: &mut dyn Database, config: &ProjectConfig) {
    for (crate_identifier, directory_path) in config.content.crate_roots.iter() {
        let root = Directory::Real(config.absolute_crate_root(directory_path));
        update_crate_root(db, config, crate_identifier, root);
    }
}

/// Updates a single crate root from a `ProjectConfig`.
/// If the crate defines settings in the config, they will be used.
/// The crate is identified by name and root directory.
pub fn update_crate_root(
    db: &mut dyn Database,
    config: &ProjectConfig,
    crate_identifier: &CrateIdentifier,
    root: Directory<'_>,
) {
    let (crate_id, crate_settings) = get_crate_id_and_settings(db, crate_identifier, config);
    set_crate_config!(
        db,
        crate_id,
        Some(CrateConfiguration { root, settings: crate_settings.clone(), cache_file: None })
    );
}

/// Sets up the DB to compile the project at the given path.
/// The path can be either a directory with a Cairo project file or a `.cairo` file.
/// Returns the IDs of the project crates.
pub fn setup_project(db: &mut dyn Database, path: &Path) -> Result<Vec<CrateInput>, ProjectError> {
    if path.is_dir() {
        let config = ProjectConfig::from_directory(path).map_err(ProjectError::LoadProjectError)?;
        let main_crate_ids: Vec<_> = get_main_crate_ids_from_project(db, &config)
            .into_iter()
            .map(|id| id.long(db).clone().into_crate_input(db))
            .collect();
        update_crate_roots_from_project_config(db, &config);
        Ok(main_crate_ids)
    } else {
        Ok(vec![setup_single_file_project(db, path)?])
    }
}

/// Sets up the DB to compile an in-memory project.
pub fn setup_in_memory_project(
    db: &mut dyn Database,
    project: &InMemoryProject,
) -> Result<Vec<CrateInput>, InMemoryProjectError> {
    if project.main_crate_name.trim().is_empty() {
        return Err(InMemoryProjectError::EmptyMainCrateName);
    }

    let core_root = build_virtual_directory(db, "core", &project.corelib_files)?;
    if !project.corelib_files.contains_key("lib.cairo") {
        return Err(InMemoryProjectError::MissingRequiredFile {
            crate_name: "core",
            path: "lib.cairo".into(),
        });
    }
    set_crate_config!(
        db,
        CrateId::core(db),
        Some(CrateConfiguration {
            root: core_root,
            settings: dev_corelib_crate_settings(),
            cache_file: None
        })
    );

    let main_root = build_virtual_directory(db, "main", &project.main_crate_files)?;
    if !project.main_crate_files.contains_key("lib.cairo") {
        return Err(InMemoryProjectError::MissingRequiredFile {
            crate_name: "main",
            path: "lib.cairo".into(),
        });
    }
    let main_crate_id = CrateId::plain(db, SmolStrId::from(db, project.main_crate_name.as_str()));
    set_crate_config!(
        db,
        main_crate_id,
        Some(CrateConfiguration {
            root: main_root,
            settings: project.main_crate_settings.clone().unwrap_or_default(),
            cache_file: None
        })
    );
    let main_crate_id = CrateId::plain(db, SmolStrId::from(db, project.main_crate_name.as_str()));

    Ok(vec![main_crate_id.long(db).clone().into_crate_input(db)])
}

/// Checks that the given path is a valid compiler path.
pub fn check_compiler_path(single_file: bool, path: &Path) -> anyhow::Result<()> {
    if path.is_file() {
        if !single_file {
            anyhow::bail!("The given path is a file, but --single-file was not supplied.");
        }
    } else if path.is_dir() {
        if single_file {
            anyhow::bail!("The given path is a directory, but --single-file was supplied.");
        }
    } else {
        anyhow::bail!("The given path does not exist.");
    }
    Ok(())
}

pub fn get_main_crate_ids_from_project<'db>(
    db: &'db dyn Database,
    config: &ProjectConfig,
) -> Vec<CrateId<'db>> {
    config
        .content
        .crate_roots
        .keys()
        .map(|crate_identifier| get_crate_id_and_settings(db, crate_identifier, config).0)
        .collect()
}

fn get_crate_id_and_settings<'db, 'a>(
    db: &'db dyn Database,
    crate_identifier: &CrateIdentifier,
    config: &'a ProjectConfig,
) -> (CrateId<'db>, &'a CrateSettings) {
    let crate_settings = config.content.crates_config.get(crate_identifier);
    let name = crate_settings.name.clone().unwrap_or_else(|| crate_identifier.clone().into());
    // It has to be done due to how `CrateId::core` works.
    let discriminator =
        if name == CORELIB_CRATE_NAME { None } else { Some(crate_identifier.clone().into()) };

    let crate_id = CrateLongId::Real { name: SmolStrId::from(db, name), discriminator }.intern(db);

    (crate_id, crate_settings)
}

#[derive(Default)]
struct VirtualDirectoryBuilder<'db> {
    files: BTreeMap<String, FileId<'db>>,
    dirs: BTreeMap<String, VirtualDirectoryBuilder<'db>>,
}

impl<'db> VirtualDirectoryBuilder<'db> {
    fn insert_file(&mut self, path_parts: &[&str], file_id: FileId<'db>) {
        if path_parts.len() == 1 {
            self.files.insert(path_parts[0].to_string(), file_id);
            return;
        }
        let (head, tail) = path_parts.split_first().unwrap();
        self.dirs.entry((*head).to_string()).or_default().insert_file(tail, file_id);
    }

    fn into_directory(self) -> Directory<'db> {
        Directory::Virtual {
            files: self.files,
            dirs: self
                .dirs
                .into_iter()
                .map(|(name, dir)| (name, Box::new(dir.into_directory())))
                .collect(),
        }
    }
}

fn build_virtual_directory<'db>(
    db: &'db dyn Database,
    crate_name: &'static str,
    files: &BTreeMap<String, String>,
) -> Result<Directory<'db>, InMemoryProjectError> {
    let mut root = VirtualDirectoryBuilder::default();
    for (path, content) in files {
        let path_parts = split_virtual_path(path).ok_or_else(|| {
            InMemoryProjectError::InvalidVirtualPath { crate_name, path: path.clone() }
        })?;
        let file_name = path_parts.last().copied().unwrap();
        let file_id = FileLongId::Virtual(VirtualFile {
            parent: None,
            name: SmolStrId::from(db, file_name),
            content: SmolStrId::from(db, content.as_str()),
            code_mappings: Vec::new().into(),
            kind: FileKind::Module,
            original_item_removed: false,
        })
        .intern(db);
        root.insert_file(&path_parts, file_id);
    }
    Ok(root.into_directory())
}

fn split_virtual_path(path: &str) -> Option<Vec<&str>> {
    if path.is_empty() || path.starts_with('/') || path.ends_with('/') {
        return None;
    }
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
        return None;
    }
    Some(parts)
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use cairo_lang_defs::db::DefsGroup;
    use cairo_lang_filesystem::db::FilesGroup;
    use cairo_lang_filesystem::ids::CrateInput;

    use super::*;
    use crate::db::RootDatabase;

    #[test]
    fn setup_in_memory_project_rejects_invalid_path() {
        let mut db = RootDatabase::builder().build().unwrap();
        let project = InMemoryProject {
            main_crate_name: "test".into(),
            main_crate_files: BTreeMap::from([("lib.cairo".into(), "fn main() {}".into())]),
            corelib_files: BTreeMap::from([
                ("lib.cairo".into(), "".into()),
                ("../bad.cairo".into(), "".into()),
            ]),
            main_crate_settings: None,
        };

        let error = setup_in_memory_project(&mut db, &project).unwrap_err();
        assert!(matches!(error, InMemoryProjectError::InvalidVirtualPath { .. }));
    }

    #[test]
    fn setup_in_memory_project_requires_lib_files() {
        let mut db = RootDatabase::builder().build().unwrap();
        let project = InMemoryProject {
            main_crate_name: "test".into(),
            main_crate_files: BTreeMap::new(),
            corelib_files: BTreeMap::new(),
            main_crate_settings: None,
        };

        let error = setup_in_memory_project(&mut db, &project).unwrap_err();
        assert!(matches!(error, InMemoryProjectError::MissingRequiredFile { .. }));
    }

    #[test]
    fn setup_in_memory_project_exposes_virtual_files() {
        let mut db = RootDatabase::builder().build().unwrap();
        let project = InMemoryProject {
            main_crate_name: "test".into(),
            main_crate_files: BTreeMap::from([
                ("lib.cairo".into(), "mod nested;".into()),
                ("nested.cairo".into(), "fn x() {}".into()),
            ]),
            corelib_files: BTreeMap::from([("lib.cairo".into(), "".into())]),
            main_crate_settings: None,
        };

        let inputs = setup_in_memory_project(&mut db, &project).unwrap();
        let main_crate_id = CrateInput::into_crate_ids(&db, inputs).into_iter().next().unwrap();
        let main_module = db.module_main_file(ModuleId::CrateRoot(main_crate_id)).unwrap();
        assert_eq!(db.file_content(main_module), Some("mod nested;"));

        let core_module = db.module_main_file(ModuleId::CrateRoot(CrateId::core(&db))).unwrap();
        assert_eq!(db.file_content(core_module), Some(""));
    }
}
