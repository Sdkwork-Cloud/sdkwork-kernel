import type { PatchOperation } from './patch-operation';

export interface PatchSet {
  patchId: string;
  workspaceId: string;
  summary: string;
  operations: PatchOperation[];
}
