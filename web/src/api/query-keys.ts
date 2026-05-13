/**
 * Centralized query key factory.
 *
 * All TanStack Query keys are defined here to make invalidation consistent
 * and prevent stale-closure key mismatches across hooks.
 */
export const queryKeys = {
  session: () => ['session'] as const,

  workspaces: () => ['workspaces'] as const,
  workspace: (slug: string) => ['workspaces', slug] as const,

  projects: (workspaceSlug: string) => ['workspaces', workspaceSlug, 'projects'] as const,
  project: (workspaceSlug: string, projectSlug: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug] as const,

  taskGroups: (workspaceSlug: string, projectSlug: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'task-groups'] as const,
  taskGroup: (workspaceSlug: string, projectSlug: string, groupId: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'task-groups', groupId] as const,

  tasks: (workspaceSlug: string, projectSlug: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'tasks'] as const,
  task: (workspaceSlug: string, projectSlug: string, taskId: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'tasks', taskId] as const,
  taskDependencies: (workspaceSlug: string, projectSlug: string, taskId: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'tasks', taskId, 'dependencies'] as const,

  notes: (workspaceSlug: string, projectSlug: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'notes'] as const,
  note: (workspaceSlug: string, projectSlug: string, noteId: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'notes', noteId] as const,

  documents: (workspaceSlug: string, projectSlug: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'documents'] as const,
  document: (workspaceSlug: string, projectSlug: string, documentId: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'documents', documentId] as const,

  assets: (workspaceSlug: string, projectSlug: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'assets'] as const,
  asset: (workspaceSlug: string, projectSlug: string, assetId: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'assets', assetId] as const,

  members: (workspaceSlug: string) => ['workspaces', workspaceSlug, 'members'] as const,

  agents: (workspaceSlug: string) => ['workspaces', workspaceSlug, 'agents'] as const,
  agent: (workspaceSlug: string, agentId: string) => ['workspaces', workspaceSlug, 'agents', agentId] as const,
  agentCredentials: (workspaceSlug: string, agentId: string) =>
    ['workspaces', workspaceSlug, 'agents', agentId, 'credentials'] as const,
  agentCredential: (workspaceSlug: string, credentialId: string) =>
    ['workspaces', workspaceSlug, 'agent-credentials', credentialId] as const,

  integrationConnections: (workspaceSlug: string) =>
    ['workspaces', workspaceSlug, 'integration-connections'] as const,
  integrationConnection: (workspaceSlug: string, connectionId: string) =>
    ['workspaces', workspaceSlug, 'integration-connections', connectionId] as const,

  workspaceActivity: (workspaceSlug: string, page: number, perPage: number) =>
    ['workspaces', workspaceSlug, 'activity', { page, perPage }] as const,
  projectActivity: (workspaceSlug: string, projectSlug: string, page: number, perPage: number) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'activity', { page, perPage }] as const,

  search: (workspaceSlug: string, projectSlug: string | undefined, query: string) =>
    ['search', { workspaceSlug, projectSlug, query }] as const,
} as const;
