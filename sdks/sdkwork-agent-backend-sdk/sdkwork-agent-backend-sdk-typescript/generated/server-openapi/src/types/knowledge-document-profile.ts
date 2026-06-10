export interface KnowledgeDocumentProfile {
  author?: string;
  content?: string;
  parentId?: string;
  type?: 'markdown' | 'file' | 'folder';
  fileName?: string;
  fileSize?: string;
  mimeType?: string;
  driveUri?: string;
}
