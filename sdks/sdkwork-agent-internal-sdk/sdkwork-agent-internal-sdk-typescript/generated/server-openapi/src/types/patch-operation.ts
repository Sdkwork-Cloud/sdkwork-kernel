export interface PatchOperation {
  kind: 'createFile' | 'updateFile' | 'deleteFile';
  path?: string;
  content?: string | null;
  before?: string | null;
  after?: string | null;
}
