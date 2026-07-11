export interface TerminalOutputChunk {
  commandId: string;
  sequence: string;
  channel: 'stdout' | 'stderr' | 'system';
  content: string;
  redactionClassification?: string | null;
}
