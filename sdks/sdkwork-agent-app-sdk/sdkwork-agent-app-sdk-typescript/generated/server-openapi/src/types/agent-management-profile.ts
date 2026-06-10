export interface AgentManagementProfile {
  author?: string;
  avatar?: string;
  categoryId?: string;
  color?: string;
  debugMode?: boolean;
  iconName?: string;
  jsonMode?: boolean;
  knowledgeBaseIds?: string[];
  memoryEnabled?: boolean;
  model?: string;
  skillIds?: string[];
  suggestedPrompts?: string[];
  systemPrompt?: string;
  temperature?: number;
  toolIds?: string[];
  type?: 'normal' | 'independent';
  users?: string;
  voiceIds?: string[];
  welcomeMessage?: string;
}
