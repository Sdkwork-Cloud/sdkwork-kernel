export interface Workspace {
  workspaceId: string;
  root: string;
  branch: string;
  dirty: boolean;
  changedFiles: string[];
}
