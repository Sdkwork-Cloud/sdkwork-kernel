use crate::ids::{SDKWORK_DRIVE_PLUGIN_ID, SDKWORK_DRIVE_PROVIDER_ID};
use sdkwork_agent_kernel::ProviderManifest;
use sdkwork_agent_plugin_core::{
    KernelPluginConformanceProfile, KernelPluginManifest, SdkworkKernelFoundationPlugin,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct SdkworkDrivePlugin;

impl SdkworkDrivePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl SdkworkKernelFoundationPlugin for SdkworkDrivePlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest {
        sdkwork_drive_plugin_manifest()
    }

    fn provider_manifests(&self) -> Vec<ProviderManifest> {
        sdkwork_drive_provider_manifests()
    }

    fn conformance_profile(&self) -> KernelPluginConformanceProfile {
        sdkwork_drive_conformance_profile()
    }
}

pub fn sdkwork_drive_plugin_manifest() -> KernelPluginManifest {
    KernelPluginManifest::new(
        SDKWORK_DRIVE_PLUGIN_ID,
        "SDKWork Drive",
        env!("CARGO_PKG_VERSION"),
        "official-foundation-plugin",
    )
    .with_provider_id(SDKWORK_DRIVE_PROVIDER_ID)
    .with_supported_profile("plugin-manifest")
    .with_supported_profile("provider-storage")
    .with_supported_profile("foundation-drive")
}

pub fn sdkwork_drive_provider_manifest() -> ProviderManifest {
    ProviderManifest::new(
        SDKWORK_DRIVE_PROVIDER_ID,
        "storage",
        "sdkwork-drive-storage-provider",
        env!("CARGO_PKG_VERSION"),
        drive_storage_capabilities(),
    )
}

pub fn sdkwork_drive_provider_manifests() -> Vec<ProviderManifest> {
    vec![sdkwork_drive_provider_manifest()]
}

pub fn sdkwork_drive_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("sdkwork-drive")
        .require_profile("plugin-manifest")
        .require_profile("provider-storage")
        .require_profile("foundation-drive")
}

fn drive_storage_capabilities() -> Vec<String> {
    [
        "drive.object.put",
        "drive.object.head",
        "drive.object.delete",
        "drive.object.list",
        "drive.object.copy",
        "drive.object.read_range",
        "drive.object.presign_download",
        "drive.bucket.head",
        "drive.bucket.list",
        "drive.bucket.create",
        "drive.bucket.delete",
        "drive.multipart.create",
        "drive.multipart.presign_upload_part",
        "drive.multipart.complete",
        "drive.multipart.abort",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
