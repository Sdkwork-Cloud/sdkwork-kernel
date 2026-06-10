use crate::manifest::sdkwork_drive_provider_manifest;
use async_trait::async_trait;
use sdkwork_agent_kernel::{ProviderHealth, ProviderManifest};
use sdkwork_drive_storage_contract::{
    AbortMultipartUploadRequest, CompleteMultipartUploadRequest, CompleteMultipartUploadResponse,
    CopyObjectRequest, CopyObjectResponse, CreateBucketRequest, CreateBucketResponse,
    CreateMultipartUploadRequest, CreateMultipartUploadResponse, DeleteBucketRequest,
    DeleteBucketResponse, DeleteObjectRequest, DeleteObjectResponse, DriveObjectChunkStream,
    DriveObjectStore, DriveObjectStoreError, DriveStorageProviderCapabilities,
    DriveStorageProviderKind, HeadBucketRequest, HeadBucketResponse, HeadObjectRequest,
    HeadObjectResponse, ListBucketsRequest, ListBucketsResponse, ListObjectsRequest,
    ListObjectsResponse, PresignDownloadRequest, PresignUploadPartRequest,
    PresignedDownloadResponse, PresignedUploadPartResponse, PutObjectRequest, PutObjectResponse,
    ReadObjectRangeRequest, ReadObjectRangeResponse,
};

pub struct SdkworkDriveStorageProvider<S> {
    object_store: S,
}

impl<S> SdkworkDriveStorageProvider<S> {
    pub fn new(object_store: S) -> Self {
        Self { object_store }
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        sdkwork_drive_provider_manifest()
    }

    pub fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    pub fn object_store(&self) -> &S {
        &self.object_store
    }

    pub fn into_inner(self) -> S {
        self.object_store
    }
}

#[async_trait]
impl<S> DriveObjectStore for SdkworkDriveStorageProvider<S>
where
    S: DriveObjectStore,
{
    fn provider_kind(&self) -> DriveStorageProviderKind {
        self.object_store.provider_kind()
    }

    fn capabilities(&self) -> DriveStorageProviderCapabilities {
        self.object_store.capabilities()
    }

    async fn put_object(
        &self,
        request: PutObjectRequest,
    ) -> Result<PutObjectResponse, DriveObjectStoreError> {
        self.object_store.put_object(request).await
    }

    async fn head_object(
        &self,
        request: HeadObjectRequest,
    ) -> Result<HeadObjectResponse, DriveObjectStoreError> {
        self.object_store.head_object(request).await
    }

    async fn delete_object(
        &self,
        request: DeleteObjectRequest,
    ) -> Result<DeleteObjectResponse, DriveObjectStoreError> {
        self.object_store.delete_object(request).await
    }

    async fn head_bucket(
        &self,
        request: HeadBucketRequest,
    ) -> Result<HeadBucketResponse, DriveObjectStoreError> {
        self.object_store.head_bucket(request).await
    }

    async fn list_buckets(
        &self,
        request: ListBucketsRequest,
    ) -> Result<ListBucketsResponse, DriveObjectStoreError> {
        self.object_store.list_buckets(request).await
    }

    async fn create_bucket(
        &self,
        request: CreateBucketRequest,
    ) -> Result<CreateBucketResponse, DriveObjectStoreError> {
        self.object_store.create_bucket(request).await
    }

    async fn delete_bucket(
        &self,
        request: DeleteBucketRequest,
    ) -> Result<DeleteBucketResponse, DriveObjectStoreError> {
        self.object_store.delete_bucket(request).await
    }

    async fn list_objects(
        &self,
        request: ListObjectsRequest,
    ) -> Result<ListObjectsResponse, DriveObjectStoreError> {
        self.object_store.list_objects(request).await
    }

    async fn copy_object(
        &self,
        request: CopyObjectRequest,
    ) -> Result<CopyObjectResponse, DriveObjectStoreError> {
        self.object_store.copy_object(request).await
    }

    async fn create_multipart_upload(
        &self,
        request: CreateMultipartUploadRequest,
    ) -> Result<CreateMultipartUploadResponse, DriveObjectStoreError> {
        self.object_store.create_multipart_upload(request).await
    }

    async fn presign_upload_part(
        &self,
        request: PresignUploadPartRequest,
    ) -> Result<PresignedUploadPartResponse, DriveObjectStoreError> {
        self.object_store.presign_upload_part(request).await
    }

    async fn complete_multipart_upload(
        &self,
        request: CompleteMultipartUploadRequest,
    ) -> Result<CompleteMultipartUploadResponse, DriveObjectStoreError> {
        self.object_store.complete_multipart_upload(request).await
    }

    async fn abort_multipart_upload(
        &self,
        request: AbortMultipartUploadRequest,
    ) -> Result<(), DriveObjectStoreError> {
        self.object_store.abort_multipart_upload(request).await
    }

    async fn presign_download(
        &self,
        request: PresignDownloadRequest,
    ) -> Result<PresignedDownloadResponse, DriveObjectStoreError> {
        self.object_store.presign_download(request).await
    }

    async fn read_object_range(
        &self,
        request: ReadObjectRangeRequest,
    ) -> Result<(ReadObjectRangeResponse, Box<dyn DriveObjectChunkStream>), DriveObjectStoreError>
    {
        self.object_store.read_object_range(request).await
    }
}
