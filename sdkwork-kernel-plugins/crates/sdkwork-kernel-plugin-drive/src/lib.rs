//! SDKWork Drive kernel plugin.

mod ids;
mod manifest;
mod provider;

pub use ids::{SDKWORK_DRIVE_PLUGIN_ID, SDKWORK_DRIVE_PROVIDER_ID};
pub use manifest::{
    sdkwork_drive_conformance_profile, sdkwork_drive_plugin_manifest,
    sdkwork_drive_provider_manifest, sdkwork_drive_provider_manifests, SdkworkDrivePlugin,
};
pub use provider::SdkworkDriveStorageProvider;
