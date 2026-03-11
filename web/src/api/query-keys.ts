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

  members: (workspaceSlug: string) => ['workspaces', workspaceSlug, 'members'] as const,

  agents: (workspaceSlug: string) => ['workspaces', workspaceSlug, 'agents'] as const,

  activity: (workspaceSlug: string, projectSlug: string) =>
    ['workspaces', workspaceSlug, 'projects', projectSlug, 'activity'] as const,
} as const;
