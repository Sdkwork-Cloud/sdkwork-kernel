//! Shared provider config-file mutation helpers.
//!
//! Provider adapters materialize applied model configurations (base URL,
//! API key, model) into the external CLI's native config surface so the CLI
//! actually uses them at request time. Every mutation follows the
//! installation/configuration spec contract: backup before mutation, atomic
//! write, read-back verification, and restoration on failure.

use sdkwork_agent_kernel::{KernelError, KernelResult};
use std::path::{Path, PathBuf};

/// Suffix of the sibling backup file created before a provider config mutation.
pub const PROVIDER_CONFIG_BACKUP_SUFFIX: &str = ".sdkwork.bak";
/// Suffix of the temporary file used for atomic writes.
const PROVIDER_CONFIG_TMP_SUFFIX: &str = ".sdkwork.tmp";

/// Resolves the sibling backup path for a provider config file.
pub fn provider_config_backup_path(config_path: &Path) -> PathBuf {
    let mut file_name = config_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(PROVIDER_CONFIG_BACKUP_SUFFIX);
    config_path.with_file_name(file_name)
}

/// Reads the current provider config content; `None` when the file is absent.
pub fn read_provider_config(path: &Path) -> KernelResult<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(KernelError::provider_error("provider_config_file", format!(
            "provider config {} could not be read: {error}",
            path.display()
        ))),
    }
}

/// Backs up the current config file (if present) into a sibling `.sdkwork.bak`.
///
/// An existing backup is never overwritten: it holds the original
/// pre-materialization state, and repeated apply calls must keep that
/// original so `dematerialize` can always restore it.
fn backup_provider_config(path: &Path) -> KernelResult<bool> {
    if provider_config_backup_path(path).is_file() {
        return Ok(true);
    }
    let Some(current) = read_provider_config(path)? else {
        return Ok(false);
    };
    let backup_path = provider_config_backup_path(path);
    write_file_atomic(&backup_path, &current)?;
    Ok(true)
}

/// Writes the provider config file atomically (temp file + rename) and
/// verifies the read-back content before reporting success.
fn write_file_atomic(path: &Path, content: &str) -> KernelResult<()> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent).map_err(|error| {
            KernelError::provider_error("provider_config_file", format!(
                "provider config directory {} could not be created: {error}",
                parent.display()
            ))
        })?;
    }
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(PROVIDER_CONFIG_TMP_SUFFIX);
    let tmp_path = path.with_file_name(file_name);
    let write_result = std::fs::write(&tmp_path, content)
        .and_then(|()| std::fs::rename(&tmp_path, path));
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(KernelError::provider_error("provider_config_file", format!(
            "provider config {} could not be written: {error}",
            path.display()
        )));
    }
    match read_provider_config(path)? {
        Some(verified) if verified == content => Ok(()),
        Some(_) => Err(KernelError::provider_error("provider_config_file", format!(
            "provider config {} write verification failed",
            path.display()
        ))),
        None => Err(KernelError::provider_error("provider_config_file", format!(
            "provider config {} is missing after write",
            path.display()
        ))),
    }
}

/// Restores the pre-mutation backup (removing the materialized content) and
/// deletes the backup file. Returns `true` when a backup was restored.
pub fn restore_provider_config_backup(path: &Path) -> KernelResult<bool> {
    let backup_path = provider_config_backup_path(path);
    match read_provider_config(&backup_path)? {
        Some(backup) => {
            write_file_atomic(path, &backup)?;
            std::fs::remove_file(&backup_path).map_err(|error| {
                KernelError::provider_error("provider_config_file", format!(
                    "provider config backup {} could not be removed: {error}",
                    backup_path.display()
                ))
            })?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Dematerializes a provider config file: restores the pre-apply backup when
/// one exists; otherwise removes the file (it did not exist before the
/// materialization).
pub fn dematerialize_provider_config(path: &Path) -> KernelResult<()> {
    if restore_provider_config_backup(path)? {
        return Ok(());
    }
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(KernelError::provider_error("provider_config_file", format!(
            "provider config {} could not be removed: {error}",
            path.display()
        ))),
    }
}

/// Mutates a provider config file with backup/verify/rollback semantics.
///
/// The `transform` callback receives the current content (`None` when the
/// file does not exist yet) and returns the next content. On any failure the
/// pre-mutation state is restored before the error is propagated.
pub fn update_provider_config_file(
    path: &Path,
    transform: impl FnOnce(Option<&str>) -> KernelResult<String>,
) -> KernelResult<()> {
    let had_backup = backup_provider_config(path)?;
    let current = read_provider_config(path)?;
    let next = match transform(current.as_deref()) {
        Ok(next) => next,
        Err(error) => {
            if had_backup {
                let _ = restore_provider_config_backup(path);
            }
            return Err(error);
        }
    };
    if let Err(error) = write_file_atomic(path, &next) {
        if had_backup {
            let _ = restore_provider_config_backup(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
        return Err(error);
    }
    Ok(())
}


/// Merges a JSON value into an existing JSON document at the given path,
/// creating intermediate objects as needed.
pub fn merge_json_path(
    document: &mut serde_json::Value,
    path: &[&str],
    value: serde_json::Value,
) {
    let mut cursor = document;
    for key in path {
        let is_object = matches!(cursor.get(*key), Some(serde_json::Value::Object(_)));
        if !is_object {
            if let Some(object) = cursor.as_object_mut() {
                object.insert((*key).to_string(), serde_json::Value::Object(Default::default()));
            }
        }
        cursor = match cursor.as_object_mut().and_then(|object| object.get_mut(*key)) {
            Some(next) => next,
            None => return,
        };
    }
    *cursor = value;
}

/// Mutates a JSON provider config file with backup/verify/rollback semantics.
/// The `transform` callback receives the parsed current document (`None` when
/// the file does not exist yet) and returns the next document.
pub fn update_provider_json_config(
    path: &Path,
    transform: impl FnOnce(Option<&serde_json::Value>) -> KernelResult<serde_json::Value>,
) -> KernelResult<()> {
    update_provider_config_file(path, |current| {
        let current = match current {
            Some(content) => Some(serde_json::from_str(content).map_err(|error| {
                KernelError::provider_error(
                    "provider_config_parse",
                    format!("{} could not be parsed as JSON: {error}", path.display()),
                )
            })?),
            None => None,
        };
        let next = transform(current.as_ref())?;
        serde_json::to_string_pretty(&next).map_err(|error| {
            KernelError::provider_error(
                "provider_config_serialize",
                format!("{} could not be serialized: {error}", path.display()),
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sdkwork-provider-config-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn update_merges_and_backs_up_existing_content() {
        let _guard = test_guard();
        let dir = temp_dir("merge");
        let path = dir.join("config.toml");
        std::fs::write(&path, "model = \"old\"\n").expect("seed");
        update_provider_config_file(&path, |current| {
            let existing = current.unwrap_or_default().to_string();
            Ok(format!("{existing}\nmodel = \"new\"\n"))
        })
        .expect("update");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("model = \"new\""));
        let backup = std::fs::read_to_string(provider_config_backup_path(&path)).expect("backup");
        assert_eq!(backup, "model = \"old\"\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_creates_missing_files_and_dematerialize_removes_them() {
        let _guard = test_guard();
        let dir = temp_dir("create");
        let path = dir.join(".env");
        update_provider_config_file(&path, |_current| Ok("KEY=value\n".to_string())).expect("write");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "KEY=value\n");
        dematerialize_provider_config(&path).expect("dematerialize");
        assert!(!path.exists(), "created config must be removed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_transform_restores_the_original_content() {
        let _guard = test_guard();
        let dir = temp_dir("rollback");
        let path = dir.join("settings.json");
        std::fs::write(&path, "{\"keep\": true}\n").expect("seed");
        let result = update_provider_config_file(&path, |_current| {
            Err(KernelError::validation("transform failed"))
        });
        assert!(result.is_err());
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "{\"keep\": true}\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_backup_reverts_materialization() {
        let _guard = test_guard();
        let dir = temp_dir("restore");
        let path = dir.join("config.toml");
        std::fs::write(&path, "original\n").expect("seed");
        update_provider_config_file(&path, |current| {
            Ok(format!("{}{}", current.unwrap_or_default(), "materialized\n"))
        })
        .expect("update");
        assert!(restore_provider_config_backup(&path).expect("restore"));
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "original\n");
        assert!(!provider_config_backup_path(&path).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repeated_updates_never_overwrite_the_original_backup() {
        let _guard = test_guard();
        let dir = temp_dir("repeat");
        let path = dir.join("config.toml");
        std::fs::write(&path, "original\n").expect("seed");
        update_provider_config_file(&path, |current| {
            Ok(format!("{}{}", current.unwrap_or_default(), "materialized-1\n"))
        })
        .expect("first update");
        update_provider_config_file(&path, |current| {
            Ok(format!("{}{}", current.unwrap_or_default(), "materialized-2\n"))
        })
        .expect("second update");
        // The backup still holds the ORIGINAL state so dematerialization can
        // restore it even after several materialized writes.
        assert_eq!(
            std::fs::read_to_string(provider_config_backup_path(&path)).expect("backup"),
            "original\n"
        );
        restore_provider_config_backup(&path).expect("restore");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "original\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
