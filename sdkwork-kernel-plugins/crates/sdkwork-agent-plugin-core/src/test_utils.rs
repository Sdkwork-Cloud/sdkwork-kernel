//! Test utilities for provider installation lifecycle contract tests.
//!
//! Available with the `test-utils` feature. The scripted executor answers
//! provider package-manager commands in FIFO order and records every command,
//! so contract tests can drive detection, install, upgrade, rollback, and
//! uninstall flows without touching a real registry or filesystem.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use sdkwork_agent_kernel::KernelResult;

use crate::{ProcessAdapterCommand, ProcessAdapterCommandExecutor, ProcessAdapterCommandOutput};

/// Command executor that answers scripted outputs in FIFO order and records
/// every executed command for assertions.
#[derive(Clone, Default)]
pub struct ScriptedCommandExecutor {
    outputs: Arc<Mutex<VecDeque<ProcessAdapterCommandOutput>>>,
    commands: Arc<Mutex<Vec<ProcessAdapterCommand>>>,
}

impl ScriptedCommandExecutor {
    pub fn with_outputs(outputs: Vec<ProcessAdapterCommandOutput>) -> Self {
        Self {
            outputs: Arc::new(Mutex::new(outputs.into())),
            commands: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn commands(&self) -> Vec<ProcessAdapterCommand> {
        self.commands.lock().expect("commands lock").clone()
    }
}

impl ProcessAdapterCommandExecutor for ScriptedCommandExecutor {
    fn execute(
        &self,
        command: &ProcessAdapterCommand,
    ) -> KernelResult<ProcessAdapterCommandOutput> {
        self.commands
            .lock()
            .expect("commands lock")
            .push(command.clone());
        self.outputs
            .lock()
            .expect("outputs lock")
            .pop_front()
            .ok_or_else(|| sdkwork_agent_kernel::KernelError::Internal {
                message: "scripted command output exhausted".to_string(),
            })
    }
}

/// npm `list --depth=0 --json` payload carrying only the installed versions.
///
/// Packages mapped to `None` are omitted from the dependency object, which the
/// installer interprets as not installed.
pub fn npm_list_payload(packages: &[(&str, Option<&str>)]) -> String {
    let entries: Vec<String> = packages
        .iter()
        .filter_map(|(package_id, version)| {
            version.map(|version| format!(r#""{package_id}":{{"version":"{version}"}}"#))
        })
        .collect();
    format!(r#"{{"dependencies":{{{}}}}}"#, entries.join(","))
}

/// npm absence payload: an empty dependency object with no error output.
pub fn npm_absent_payload() -> String {
    r#"{"dependencies":{}}"#.to_string()
}

/// Python metadata probe payload mapping package ids to installed versions or
/// `null` when the package is not installed.
pub fn pypi_metadata_payload(packages: &[(&str, Option<&str>)]) -> String {
    let entries: Vec<String> = packages
        .iter()
        .map(|(package_id, version)| match version {
            Some(version) => format!(r#""{package_id}":"{version}""#),
            None => format!(r#""{package_id}":null"#),
        })
        .collect();
    format!("{{{}}}", entries.join(","))
}

/// Creates a unique temporary install root for lifecycle tests.
pub fn temporary_install_root(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "sdkwork-provider-test-{prefix}-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("temporary provider install root is created");
    path
}

/// Temporary install root that removes its directory on drop.
pub struct TemporaryInstallRoot(PathBuf);

impl TemporaryInstallRoot {
    pub fn new(prefix: &str) -> Self {
        Self(temporary_install_root(prefix))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryInstallRoot {
    fn drop(&mut self) {
        let temp = std::env::temp_dir();
        let safe_name = self
            .0
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("sdkwork-provider-test-"));
        if self.0.starts_with(&temp) && safe_name {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
