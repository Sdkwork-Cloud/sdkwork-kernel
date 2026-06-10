use async_trait::async_trait;
use sdkwork_agent_plugin_core::SdkworkKernelFoundationPlugin;
use sdkwork_drive_storage_contract::{
    AbortMultipartUploadRequest, CompleteMultipartUploadRequest, CompleteMultipartUploadResponse,
    CopyObjectRequest, CopyObjectResponse, CreateBucketRequest, CreateBucketResponse,
    CreateMultipartUploadRequest, CreateMultipartUploadResponse, DeleteBucketRequest,
    DeleteBucketResponse, DeleteObjectRequest, DeleteObjectResponse, DriveObjectChunkStream,
    DriveObjectLocator, DriveObjectStore, DriveObjectStoreError, DriveObjectStoreErrorKind,
    DriveStorageProviderCapabilities, DriveStorageProviderKind, HeadBucketRequest,
    HeadBucketResponse, HeadObjectRequest, HeadObjectResponse, ListBucketsRequest,
    ListBucketsResponse, ListObjectsRequest, ListObjectsResponse, PresignDownloadRequest,
    PresignUploadPartRequest, PresignedDownloadResponse, PresignedUploadPartResponse,
    PutObjectRequest, PutObjectResponse, ReadObjectRangeRequest, ReadObjectRangeResponse,
};
use sdkwork_kernel_plugin_drive::{
    sdkwork_drive_plugin_manifest, sdkwork_drive_provider_manifests, SdkworkDrivePlugin,
    SdkworkDriveStorageProvider, SDKWORK_DRIVE_PLUGIN_ID, SDKWORK_DRIVE_PROVIDER_ID,
};

#[test]
fn plugin_manifest_declares_optional_foundation_storage_provider() {
    let manifest = sdkwork_drive_plugin_manifest();

    assert_eq!(manifest.plugin_id, SDKWORK_DRIVE_PLUGIN_ID);
    assert_eq!(manifest.implementation_kind, "official-foundation-plugin");
    assert_eq!(manifest.agent_id, None);
    assert_eq!(
        manifest.provider_ids,
        [SDKWORK_DRIVE_PROVIDER_ID.to_string()]
    );
    assert!(manifest.supports_profile("provider-storage"));
    assert!(manifest.supports_profile("foundation-drive"));
}

#[test]
fn foundation_plugin_trait_exposes_drive_provider_without_agent_manifest() {
    let plugin = SdkworkDrivePlugin::new();
    assert_foundation_plugin_trait(&plugin);

    assert_eq!(plugin.plugin_manifest().plugin_id, SDKWORK_DRIVE_PLUGIN_ID);
    assert_eq!(
        plugin.provider_manifests()[0].provider_id,
        SDKWORK_DRIVE_PROVIDER_ID
    );
    assert!(plugin.conformance_profile().requires("provider-storage"));
    assert!(plugin.conformance_profile().requires("foundation-drive"));
}

#[test]
fn provider_manifest_declares_standard_drive_storage_capabilities() {
    let provider = SdkworkDriveStorageProvider::new(FakeDriveObjectStore);

    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_id, SDKWORK_DRIVE_PROVIDER_ID);
    assert_eq!(manifest.provider_family, "storage");
    for capability in [
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
    ] {
        assert!(
            manifest.capabilities.contains(&capability.to_string()),
            "provider manifest should expose {capability}"
        );
    }
    assert_eq!(sdkwork_drive_provider_manifests(), vec![manifest]);
}

#[tokio::test]
async fn drive_storage_provider_wraps_drive_object_store_contract() {
    let provider = SdkworkDriveStorageProvider::new(FakeDriveObjectStore);

    assert_eq!(
        provider.provider_kind(),
        DriveStorageProviderKind::LocalFilesystem
    );
    assert_eq!(
        provider.capabilities(),
        DriveStorageProviderCapabilities::default_local_filesystem()
    );

    let locator = DriveObjectLocator {
        bucket: "agent-artifacts".to_string(),
        object_key: "runs/run-1/report.md".to_string(),
    };
    let put_response = provider
        .put_object(PutObjectRequest {
            locator: locator.clone(),
            content_type: Some("text/markdown".to_string()),
            metadata: Default::default(),
            body: b"report".to_vec(),
            checksum_sha256_hex: None,
        })
        .await
        .unwrap();
    let head_response = provider
        .head_object(HeadObjectRequest {
            locator: locator.clone(),
        })
        .await
        .unwrap();

    assert_eq!(put_response.locator, locator);
    assert_eq!(put_response.etag.as_deref(), Some("etag-report"));
    assert_eq!(head_response.content_length, 6);
    assert_eq!(head_response.content_type.as_deref(), Some("text/markdown"));
}

fn assert_foundation_plugin_trait<T: SdkworkKernelFoundationPlugin>(_plugin: &T) {}

struct FakeDriveObjectStore;

#[async_trait]
impl DriveObjectStore for FakeDriveObjectStore {
    fn provider_kind(&self) -> DriveStorageProviderKind {
        DriveStorageProviderKind::LocalFilesystem
    }

    fn capabilities(&self) -> DriveStorageProviderCapabilities {
        DriveStorageProviderCapabilities::default_local_filesystem()
    }

    async fn put_object(
        &self,
        request: PutObjectRequest,
    ) -> Result<PutObjectResponse, DriveObjectStoreError> {
        Ok(PutObjectResponse {
            locator: request.locator,
            etag: Some("etag-report".to_string()),
            version_id: None,
        })
    }

    async fn head_object(
        &self,
        request: HeadObjectRequest,
    ) -> Result<HeadObjectResponse, DriveObjectStoreError> {
        Ok(HeadObjectResponse {
            locator: request.locator,
            content_length: 6,
            content_type: Some("text/markdown".to_string()),
            etag: Some("etag-report".to_string()),
            version_id: None,
            checksum_sha256_hex: None,
            metadata: Default::default(),
        })
    }

    async fn delete_object(
        &self,
        request: DeleteObjectRequest,
    ) -> Result<DeleteObjectResponse, DriveObjectStoreError> {
        Ok(DeleteObjectResponse {
            locator: request.locator,
            deleted: true,
        })
    }

    async fn head_bucket(
        &self,
        request: HeadBucketRequest,
    ) -> Result<HeadBucketResponse, DriveObjectStoreError> {
        Ok(HeadBucketResponse {
            bucket: request.bucket,
            exists: true,
        })
    }

    async fn list_buckets(
        &self,
        _request: ListBucketsRequest,
    ) -> Result<ListBucketsResponse, DriveObjectStoreError> {
        Ok(ListBucketsResponse { items: Vec::new() })
    }

    async fn create_bucket(
        &self,
        request: CreateBucketRequest,
    ) -> Result<CreateBucketResponse, DriveObjectStoreError> {
        Ok(CreateBucketResponse {
            bucket: request.bucket,
            created: true,
        })
    }

    async fn delete_bucket(
        &self,
        request: DeleteBucketRequest,
    ) -> Result<DeleteBucketResponse, DriveObjectStoreError> {
        Ok(DeleteBucketResponse {
            bucket: request.bucket,
            deleted: true,
        })
    }

    async fn list_objects(
        &self,
        request: ListObjectsRequest,
    ) -> Result<ListObjectsResponse, DriveObjectStoreError> {
        Ok(ListObjectsResponse {
            bucket: request.bucket,
            prefix: request.prefix,
            items: Vec::new(),
            next_continuation_token: None,
            is_truncated: false,
        })
    }

    async fn copy_object(
        &self,
        request: CopyObjectRequest,
    ) -> Result<CopyObjectResponse, DriveObjectStoreError> {
        Ok(CopyObjectResponse {
            locator: request.destination,
            etag: Some("etag-copy".to_string()),
            version_id: None,
        })
    }

    async fn create_multipart_upload(
        &self,
        request: CreateMultipartUploadRequest,
    ) -> Result<CreateMultipartUploadResponse, DriveObjectStoreError> {
        Ok(CreateMultipartUploadResponse {
            locator: request.locator,
            upload_id: "upload-1".to_string(),
        })
    }

    async fn presign_upload_part(
        &self,
        _request: PresignUploadPartRequest,
    ) -> Result<PresignedUploadPartResponse, DriveObjectStoreError> {
        Err(not_supported())
    }

    async fn complete_multipart_upload(
        &self,
        request: CompleteMultipartUploadRequest,
    ) -> Result<CompleteMultipartUploadResponse, DriveObjectStoreError> {
        Ok(CompleteMultipartUploadResponse {
            locator: request.locator,
            etag: Some("etag-complete".to_string()),
            version_id: None,
        })
    }

    async fn abort_multipart_upload(
        &self,
        _request: AbortMultipartUploadRequest,
    ) -> Result<(), DriveObjectStoreError> {
        Ok(())
    }

    async fn presign_download(
        &self,
        _request: PresignDownloadRequest,
    ) -> Result<PresignedDownloadResponse, DriveObjectStoreError> {
        Err(not_supported())
    }

    async fn read_object_range(
        &self,
        _request: ReadObjectRangeRequest,
    ) -> Result<(ReadObjectRangeResponse, Box<dyn DriveObjectChunkStream>), DriveObjectStoreError>
    {
        Err(not_supported())
    }
}

fn not_supported() -> DriveObjectStoreError {
    DriveObjectStoreError::new(DriveObjectStoreErrorKind::NotSupported, "not supported")
}
