use super::*;
use std::sync::Barrier;

const LOCK_HELPER_ROOT_ENV: &str = "SDKWORK_TEST_PROVIDER_LOCK_ROOT";
const LOCK_HELPER_MODE_ENV: &str = "SDKWORK_TEST_PROVIDER_LOCK_MODE";
const LOCK_HELPER_READY_ENV: &str = "SDKWORK_TEST_PROVIDER_LOCK_READY";
const LOCK_HELPER_TEST_NAME: &str = "process_adapter::tests::lifecycle_lock_helper";
static ENVIRONMENT_TEST_LOCK: Mutex<()> = Mutex::new(());

struct EnvironmentVariableGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

struct BarrierReader {
    barrier: Arc<Barrier>,
}

impl Read for BarrierReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        self.barrier.wait();
        Ok(0)
    }
}

impl EnvironmentVariableGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvironmentVariableGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

struct LockHelper {
    child: Child,
    lock_path: PathBuf,
    ready_directory: PathBuf,
}

impl LockHelper {
    fn spawn(label: &str, mode: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let ready_directory = std::env::temp_dir().join(format!(
            "sdkwork-provider-lock-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&ready_directory).expect("create lifecycle lock contract directory");
        let ready_path = ready_directory.join("ready");
        let runtime_root = ready_directory.join("provider-runtime");
        let installer = lifecycle_lock_test_installer(&runtime_root);
        let key = installer
            .operation_lock_key()
            .expect("resolve lifecycle lock key");
        let lock_path = operation_lock_file_path(&key).expect("resolve lifecycle lock path");
        let child = Command::new(std::env::current_exe().expect("resolve test executable"))
            .args(["--exact", LOCK_HELPER_TEST_NAME, "--nocapture"])
            .env(LOCK_HELPER_ROOT_ENV, &runtime_root)
            .env(LOCK_HELPER_MODE_ENV, mode)
            .env(LOCK_HELPER_READY_ENV, &ready_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn lifecycle lock helper");
        let mut helper = Self {
            child,
            lock_path,
            ready_directory,
        };
        helper.wait_until_ready(&ready_path);
        helper
    }

    fn wait_until_ready(&mut self, ready_path: &Path) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !ready_path.is_file() {
            if let Some(status) = self.child.try_wait().expect("poll lifecycle lock helper") {
                panic!("lifecycle lock helper exited before readiness: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "lifecycle lock helper readiness timed out"
            );
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn wait_for_success(&mut self) {
        let status = self.child.wait().expect("wait for lifecycle lock helper");
        assert!(status.success());
    }
}

impl Drop for LockHelper {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = fs::remove_file(&self.lock_path);
        let _ = fs::remove_dir_all(&self.ready_directory);
    }
}

fn lifecycle_lock_test_installer(runtime_root: &Path) -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        "agent.intelligence.lifecycle-lock-contract",
        "provider.agent.installer.lifecycle-lock-contract",
        "1.0.0",
        ProcessAdapterPackage::npm("@sdkwork/lifecycle-lock-contract", "1.0.0"),
    )
    .with_install_root(runtime_root.to_path_buf())
}

#[test]
fn missing_install_roots_use_the_canonical_existing_ancestor() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "sdkwork-provider-root-resolution-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&base).expect("create install root resolution base");
    let root = base.join("missing").join("provider-runtime");

    let resolved = resolve_install_root(&root).expect("resolve missing install root");
    let expected = dunce::canonicalize(&base)
        .expect("canonicalize install root base")
        .join("missing")
        .join("provider-runtime");

    assert_eq!(resolved, expected);
    fs::remove_dir_all(&base).expect("remove install root resolution base");
}

#[test]
fn executable_lock_identity_uses_the_canonical_executable_path() {
    let executable = std::env::current_exe().expect("resolve current test executable");
    let expected = dunce::canonicalize(&executable).expect("canonicalize current test executable");

    assert_eq!(
        executable_lock_identity(executable.to_string_lossy().as_ref()),
        expected.to_string_lossy()
    );
}

#[test]
fn lifecycle_lock_artifacts_use_a_bounded_shard_pool() {
    let paths: HashSet<PathBuf> = (0..OPERATION_LOCK_SHARD_COUNT * 2)
        .map(|index| {
            operation_lock_file_path(&format!("npm:runtime-{index}"))
                .expect("resolve lifecycle lock shard")
        })
        .collect();

    assert!(paths.len() <= OPERATION_LOCK_SHARD_COUNT as usize);
    assert!(paths.iter().all(|path| {
        path.file_stem()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.len() == 3 && name.chars().all(|character| character.is_ascii_hexdigit())
            })
    }));
}

#[test]
fn explicitly_empty_runtime_environment_values_fail_closed() {
    let _environment_lock = ENVIRONMENT_TEST_LOCK
        .lock()
        .expect("lock provider environment contract");
    let python_guard = EnvironmentVariableGuard::set(PYTHON_BINARY_ENV, "");
    let python_installer = ProcessAdapterInstaller::new(
        "agent.intelligence.environment-python-contract",
        "provider.agent.installer.environment-python-contract",
        "1.0.0",
        ProcessAdapterPackage::pypi("environment-python-contract", "1.0.0"),
    );
    let python_error = python_installer
        .detect_installation("agent.intelligence.environment-python-contract")
        .expect_err("empty Python runtime configuration must fail closed");
    assert_eq!(
        python_error.kind(),
        sdkwork_agent_kernel::KernelErrorKind::ValidationError
    );
    drop(python_guard);

    let root_guard = EnvironmentVariableGuard::set(PROVIDER_RUNTIME_ROOT_ENV, "");
    let npm_installer = ProcessAdapterInstaller::new(
        "agent.intelligence.environment-npm-contract",
        "provider.agent.installer.environment-npm-contract",
        "1.0.0",
        ProcessAdapterPackage::npm("@sdkwork/environment-npm-contract", "1.0.0"),
    );
    let npm_error = npm_installer
        .detect_installation("agent.intelligence.environment-npm-contract")
        .expect_err("empty npm runtime root configuration must fail closed");
    assert_eq!(
        npm_error.kind(),
        sdkwork_agent_kernel::KernelErrorKind::ValidationError
    );
    drop(root_guard);
}

#[test]
fn output_reader_capacity_is_bounded_and_recovers_after_readers_exit() {
    let barrier = Arc::new(Barrier::new(MAX_ACTIVE_OUTPUT_READERS + 1));
    let readers: Vec<_> = (0..MAX_ACTIVE_OUTPUT_READERS)
        .map(|_| {
            BoundedReader::spawn(BarrierReader {
                barrier: Arc::clone(&barrier),
            })
            .expect("reserve bounded output reader")
        })
        .collect();
    let error = match BoundedReader::spawn(std::io::Cursor::new(Vec::<u8>::new())) {
        Ok(_) => panic!("output reader capacity must be bounded"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "provider_installer_output_capacity_exhausted");

    barrier.wait();
    let deadline = Instant::now() + Duration::from_secs(5);
    while ACTIVE_OUTPUT_READERS.load(Ordering::Acquire) != 0 {
        assert!(
            Instant::now() < deadline,
            "output reader capacity did not recover"
        );
        thread::sleep(Duration::from_millis(10));
    }
    for reader in readers {
        reader.finish().expect("finish bounded output reader");
    }
}

#[cfg(windows)]
#[test]
fn windows_npm_resolution_rejects_split_node_and_cli_directories() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "sdkwork-provider-npm-resolution-{}-{nonce}",
        std::process::id()
    ));
    let node_directory = base.join("node");
    let npm_directory = base.join("npm");
    fs::create_dir_all(&node_directory).expect("create fake Node directory");
    let npm_cli = npm_directory
        .join("node_modules")
        .join("npm")
        .join("bin")
        .join("npm-cli.js");
    fs::create_dir_all(npm_cli.parent().expect("fake npm CLI parent"))
        .expect("create fake npm CLI directory");
    fs::write(node_directory.join("node.exe"), b"fake-node").expect("write fake Node executable");
    fs::write(&npm_cli, b"fake-npm").expect("write fake npm CLI");
    let split_path =
        std::env::join_paths([&node_directory, &npm_directory]).expect("join split Node/npm path");

    let error = resolve_windows_npm_runtime_from_path(&split_path)
        .expect_err("Node and npm from different directories must be rejected");
    assert_eq!(error.code(), "provider_installer_command_unavailable");

    let colocated_cli = node_directory
        .join("node_modules")
        .join("npm")
        .join("bin")
        .join("npm-cli.js");
    fs::create_dir_all(colocated_cli.parent().expect("colocated npm CLI parent"))
        .expect("create colocated npm CLI directory");
    fs::write(&colocated_cli, b"fake-npm").expect("write colocated npm CLI");
    let colocated_path = std::env::join_paths([&node_directory]).expect("join colocated path");
    let (node, npm) = resolve_windows_npm_runtime_from_path(&colocated_path)
        .expect("colocated Node and npm resolve");
    assert_eq!(
        node,
        dunce::canonicalize(node_directory.join("node.exe")).unwrap()
    );
    assert_eq!(npm, dunce::canonicalize(&colocated_cli).unwrap());

    fs::remove_dir_all(&base).expect("remove fake Node/npm runtime");
}

#[test]
fn lifecycle_lock_coordinates_across_processes_and_releases_on_exit() {
    let mut helper = LockHelper::spawn("exclusive-contract", "exclusive");
    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&helper.lock_path)
        .expect("open lifecycle lock contender");
    let error = contender
        .try_lock_shared()
        .expect_err("child installer must hold the exclusive lifecycle lock");
    assert!(matches!(error, fs::TryLockError::WouldBlock));

    helper.wait_for_success();
    contender
        .try_lock()
        .expect("lifecycle lock must release when the installer process exits");
    contender.unlock().expect("unlock lifecycle lock contender");
}

#[test]
fn lifecycle_detection_locks_are_shared_across_processes() {
    let mut helper = LockHelper::spawn("shared-contract", "shared");
    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&helper.lock_path)
        .expect("open shared lifecycle lock contender");
    contender
        .try_lock_shared()
        .expect("detections in separate processes must share the lifecycle lock");
    contender.unlock().expect("unlock shared lifecycle lock");
    let error = contender
        .try_lock()
        .expect_err("mutation lock must wait for cross-process detection");
    assert!(matches!(error, fs::TryLockError::WouldBlock));

    helper.wait_for_success();
    contender
        .try_lock()
        .expect("exclusive lifecycle lock must recover after detection exits");
    contender.unlock().expect("unlock lifecycle lock contender");
}

#[test]
fn lifecycle_lock_helper() {
    let Some(runtime_root) = std::env::var_os(LOCK_HELPER_ROOT_ENV) else {
        return;
    };
    let ready_path =
        PathBuf::from(std::env::var_os(LOCK_HELPER_READY_ENV).expect("lock helper readiness path"));
    let installer = lifecycle_lock_test_installer(Path::new(&runtime_root));
    let hold_lock = || {
        fs::write(&ready_path, b"ready").expect("publish lifecycle lock readiness");
        thread::sleep(Duration::from_secs(2));
        Ok(())
    };
    if std::env::var(LOCK_HELPER_MODE_ENV).as_deref() == Ok("shared") {
        installer
            .with_detection_lock(hold_lock)
            .expect("acquire installer detection lock");
    } else {
        installer
            .with_mutation_lock(hold_lock)
            .expect("acquire installer mutation lock");
    }
}
