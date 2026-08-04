//! Shared provider config-file mutation helpers.
//!
//! Provider adapters materialize applied model configurations (base URL,
//! API key, model) into the external CLI's native config surface so the CLI
//! actually uses them at request time. Every mutation follows the
//! installation/configuration spec contract: backup before mutation, atomic
//! write, read-back verification, and restoration on failure.

use sdkwork_agent_kernel::{KernelError, KernelResult};
use std::path::{Path, PathBuf};

/// Suffix of the sibling backup file created before a provider config mutation
/// (default scope; provider-scoped backups use `.sdkwork.{scope}.bak`).
pub const PROVIDER_CONFIG_BACKUP_SUFFIX: &str = ".sdkwork.bak";
/// Suffix of the temporary file used for atomic writes.
const PROVIDER_CONFIG_TMP_SUFFIX: &str = ".sdkwork.tmp";

/// Resolves the sibling backup path for a provider config file (default scope).
pub fn provider_config_backup_path(config_path: &Path) -> PathBuf {
    provider_config_backup_path_named(config_path, "provider")
}

/// Resolves a provider-scoped sibling backup path.
///
/// Multiple providers may share one config surface (Claude Code and Mimo Code
/// both manage `~/.claude/settings.json`). Scoping the backup by provider
/// keeps each provider's pre-materialization state independent so one
/// provider's `dematerialize` can never restore over or delete another
/// provider's backup.
pub fn provider_config_backup_path_named(config_path: &Path, provider_scope: &str) -> PathBuf {
    let mut file_name = config_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(format!(".sdkwork.{provider_scope}.bak"));
    config_path.with_file_name(file_name)
}

/// Reads the current provider config content; `None` when the file is absent.
pub fn read_provider_config(path: &Path) -> KernelResult<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(KernelError::provider_error(
            "provider_config_file",
            format!(
                "provider config {} could not be read: {error}",
                path.display()
            ),
        )),
    }
}

/// Backs up the current config file into the provider-scoped backup path.
///
/// An existing backup (for the same provider scope) is never overwritten: it
/// holds the original pre-materialization state, and repeated apply calls must
/// keep that original so `dematerialize` can always restore it. When the
/// config file does not exist, an empty backup file is written as a marker so
/// `dematerialize` can tell "the file was created by materialization" apart
/// from "the file is user-owned"; restoring an empty backup removes the file.
fn backup_provider_config(config_path: &Path, backup_path: &Path) -> KernelResult<()> {
    if backup_path.is_file() {
        return Ok(());
    }
    let current = read_provider_config(config_path)?;
    write_file_atomic(backup_path, current.as_deref().unwrap_or(""))?;
    Ok(())
}

/// Writes the provider config file atomically (temp file + rename) and
/// verifies the read-back content before reporting success.
fn write_file_atomic(path: &Path, content: &str) -> KernelResult<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent).map_err(|error| {
            KernelError::provider_error(
                "provider_config_file",
                format!(
                    "provider config directory {} could not be created: {error}",
                    parent.display()
                ),
            )
        })?;
    }
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(PROVIDER_CONFIG_TMP_SUFFIX);
    let tmp_path = path.with_file_name(file_name);
    let write_result =
        std::fs::write(&tmp_path, content).and_then(|()| std::fs::rename(&tmp_path, path));
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(KernelError::provider_error(
            "provider_config_file",
            format!(
                "provider config {} could not be written: {error}",
                path.display()
            ),
        ));
    }
    match read_provider_config(path)? {
        Some(verified) if verified == content => Ok(()),
        Some(_) => Err(KernelError::provider_error(
            "provider_config_file",
            format!(
                "provider config {} write verification failed",
                path.display()
            ),
        )),
        None => Err(KernelError::provider_error(
            "provider_config_file",
            format!("provider config {} is missing after write", path.display()),
        )),
    }
}

/// Restores the pre-mutation backup (removing the materialized content) and
/// deletes the backup file. An empty backup marks "the config file did not
/// exist before materialization", in which case the config file itself is
/// removed. Returns `true` when a backup was restored.
pub fn restore_provider_config_backup(path: &Path) -> KernelResult<bool> {
    let backup_path = provider_config_backup_path(path);
    restore_provider_config_backup_named(path, &backup_path)
}

/// Restores the pre-mutation backup at an explicit provider-scoped path.
pub fn restore_provider_config_backup_named(path: &Path, backup_path: &Path) -> KernelResult<bool> {
    match read_provider_config(backup_path)? {
        Some(backup) => {
            if backup.is_empty() {
                // Empty backup marks "no config file before materialization":
                // remove the file created by the materialization.
                match std::fs::remove_file(path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(KernelError::provider_error(
                            "provider_config_file",
                            format!(
                                "provider config {} could not be removed while restoring: {error}",
                                path.display()
                            ),
                        ));
                    }
                }
            } else {
                write_file_atomic(path, &backup)?;
            }
            std::fs::remove_file(backup_path).map_err(|error| {
                KernelError::provider_error(
                    "provider_config_file",
                    format!(
                        "provider config backup {} could not be removed: {error}",
                        backup_path.display()
                    ),
                )
            })?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Dematerializes a provider config file at an explicit provider-scoped
/// backup path: restores the pre-apply backup when one exists.
///
/// Without a provider-scoped backup the config file is **never removed**: it
/// may be user-owned or materialized by another provider sharing the surface,
/// and deleting it would destroy user data (fail-closed). Only an absent file
/// is an unconditional no-op.
pub fn dematerialize_provider_config_named(path: &Path, provider_scope: &str) -> KernelResult<()> {
    let backup_path = provider_config_backup_path_named(path, provider_scope);
    if restore_provider_config_backup_named(path, &backup_path)? {
        return Ok(());
    }
    // No provider-scoped backup exists. This provider either never
    // materialized this surface or its backup is gone; either way the file is
    // not ours to delete.
    Ok(())
}

/// Dematerializes a provider config file: restores the pre-apply backup when
/// one exists. Without a backup the file is never removed (fail-closed; it may
/// be user-owned or managed by another provider).
pub fn dematerialize_provider_config(path: &Path) -> KernelResult<()> {
    dematerialize_provider_config_named(path, "provider")
}

/// Mutates a provider config file with backup/verify/rollback semantics using
/// the provider-scoped backup path.
///
/// The `transform` callback receives the current content (`None` when the
/// file does not exist yet) and returns the next content. On any failure the
/// pre-mutation state is restored before the error is propagated.
pub fn update_provider_config_file_named(
    path: &Path,
    provider_scope: &str,
    transform: impl FnOnce(Option<&str>) -> KernelResult<String>,
) -> KernelResult<()> {
    let backup_path = provider_config_backup_path_named(path, provider_scope);
    backup_provider_config(path, &backup_path)?;
    let current = read_provider_config(path)?;
    let next = match transform(current.as_deref()) {
        Ok(next) => next,
        Err(error) => {
            let _ = restore_provider_config_backup_named(path, &backup_path);
            return Err(error);
        }
    };
    if let Err(error) = write_file_atomic(path, &next) {
        let _ = restore_provider_config_backup_named(path, &backup_path);
        return Err(error);
    }
    Ok(())
}

/// Mutates a provider config file with backup/verify/rollback semantics using
/// the default-scope backup path.
pub fn update_provider_config_file(
    path: &Path,
    transform: impl FnOnce(Option<&str>) -> KernelResult<String>,
) -> KernelResult<()> {
    update_provider_config_file_named(path, "provider", transform)
}

/// Merges a JSON value into an existing JSON document at the given path,
/// creating intermediate objects as needed.
///
/// Fails closed when the document root (or an intermediate path element) is
/// not an object — silently skipping the merge would report materialization
/// success while the native config was never actually updated.
pub fn merge_json_path(
    document: &mut serde_json::Value,
    path: &[&str],
    value: serde_json::Value,
) -> KernelResult<()> {
    let mut cursor = document;
    for key in path {
        let is_object = matches!(cursor.get(*key), Some(serde_json::Value::Object(_)));
        if !is_object {
            if let Some(object) = cursor.as_object_mut() {
                object.insert(
                    (*key).to_string(),
                    serde_json::Value::Object(Default::default()),
                );
            }
        }
        cursor = match cursor
            .as_object_mut()
            .and_then(|object| object.get_mut(*key))
        {
            Some(next) => next,
            None => {
                return Err(KernelError::provider_error(
                    "provider_config_merge",
                    format!(
                        "cannot merge JSON config: path element `{key}` cannot be created because the document is not an object"
                    ),
                ));
            }
        };
    }
    *cursor = value;
    Ok(())
}

/// Mutates a JSON provider config file with backup/verify/rollback semantics
/// using the provider-scoped backup path. The `transform` callback receives
/// the parsed current document (`None` when the file does not exist yet) and
/// returns the next document.
pub fn update_provider_json_config_named(
    path: &Path,
    provider_scope: &str,
    transform: impl FnOnce(Option<&serde_json::Value>) -> KernelResult<serde_json::Value>,
) -> KernelResult<()> {
    update_provider_config_file_named(path, provider_scope, |current| {
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

/// Mutates a JSON provider config file with backup/verify/rollback semantics
/// using the default-scope backup path.
pub fn update_provider_json_config(
    path: &Path,
    transform: impl FnOnce(Option<&serde_json::Value>) -> KernelResult<serde_json::Value>,
) -> KernelResult<()> {
    update_provider_json_config_named(path, "provider", transform)
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
        update_provider_config_file(&path, |_current| Ok("KEY=value\n".to_string()))
            .expect("write");
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
            Ok(format!(
                "{}{}",
                current.unwrap_or_default(),
                "materialized\n"
            ))
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
            Ok(format!(
                "{}{}",
                current.unwrap_or_default(),
                "materialized-1\n"
            ))
        })
        .expect("first update");
        update_provider_config_file(&path, |current| {
            Ok(format!(
                "{}{}",
                current.unwrap_or_default(),
                "materialized-2\n"
            ))
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

    #[test]
    fn dematerialize_never_removes_a_file_without_backup() {
        let _guard = test_guard();
        let dir = temp_dir("nobackup");
        let path = dir.join("settings.json");
        std::fs::write(&path, "user-owned\n").expect("seed");
        dematerialize_provider_config(&path).expect("dematerialize is safe no-op");
        assert!(path.exists(), "user file must survive without a backup");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "user-owned\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materialized_file_is_removed_when_it_did_not_exist_before() {
        let _guard = test_guard();
        let dir = temp_dir("created-marker");
        let path = dir.join(".env");
        update_provider_config_file(&path, |_current| Ok("KEY=value\n".to_string()))
            .expect("write");
        assert!(path.exists());
        dematerialize_provider_config(&path).expect("dematerialize");
        assert!(
            !path.exists(),
            "file created by materialization must be removed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn provider_scoped_backups_are_independent_on_a_shared_surface() {
        let _guard = test_guard();
        let dir = temp_dir("shared-surface");
        let path = dir.join("settings.json");
        std::fs::write(&path, "original\n").expect("seed");

        // claude-code materializes the shared settings surface.
        update_provider_config_file_named(&path, "claude-code", |current| {
            Ok(format!(
                "{}{}",
                current.unwrap_or_default(),
                "claude-config\n"
            ))
        })
        .expect("claude update");
        // mimo-code materializes the same surface; its backup is independent.
        update_provider_config_file_named(&path, "mimo-code", |current| {
            Ok(format!(
                "{}{}",
                current.unwrap_or_default(),
                "mimo-config\n"
            ))
        })
        .expect("mimo update");

        let claude_backup = provider_config_backup_path_named(&path, "claude-code");
        let mimo_backup = provider_config_backup_path_named(&path, "mimo-code");
        assert!(claude_backup.is_file());
        assert!(mimo_backup.is_file());
        assert_eq!(
            std::fs::read_to_string(&claude_backup).expect("claude backup"),
            "original\n"
        );
        assert_eq!(
            std::fs::read_to_string(&mimo_backup).expect("mimo backup"),
            "original\nclaude-config\n"
        );

        // claude dematerialize restores its own snapshot and never touches the
        // mimo backup.
        dematerialize_provider_config_named(&path, "claude-code").expect("claude dematerialize");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "original\n");
        assert!(!claude_backup.exists());
        assert!(
            mimo_backup.is_file(),
            "mimo backup survives claude dematerialize"
        );

        // mimo dematerialize restores its own snapshot (the claude-materialized
        // state) — the user file is never deleted.
        dematerialize_provider_config_named(&path, "mimo-code").expect("mimo dematerialize");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "original\nclaude-config\n"
        );
        assert!(!mimo_backup.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_json_path_fails_closed_on_non_object_root() {
        let _guard = test_guard();
        let mut document = serde_json::json!([1, 2, 3]);
        let result = merge_json_path(&mut document, &["env"], serde_json::json!({}));
        assert!(result.is_err(), "non-object root must fail closed");
        assert_eq!(
            document,
            serde_json::json!([1, 2, 3]),
            "document must be untouched"
        );
    }

    #[test]
    fn merge_json_path_creates_intermediate_objects() {
        let _guard = test_guard();
        let mut document = serde_json::json!({});
        merge_json_path(
            &mut document,
            &["providers", "sdkwork", "model"],
            serde_json::json!("m"),
        )
        .expect("merge");
        assert_eq!(
            document["providers"]["sdkwork"]["model"],
            serde_json::json!("m")
        );
    }
}
