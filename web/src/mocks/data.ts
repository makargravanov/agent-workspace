import type {
  NoteDetail,
  ProjectSummary,
  Session,
  TaskDetail,
  TaskGroupSummary,
  WorkspaceSummary,
} from '../api/types';

export const mockSession: Session = {
  user: {
    id: '00000000-0000-0000-0000-000000000001',
    name: 'Dev User',
    email: 'dev@example.local',
    avatar_url: null,
  },
  workspace_id: '00000000-0000-0000-0000-000000000010',
};

export const mockWorkspaces: WorkspaceSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000010',
    slug: 'core-platform',
    name: 'Core Platform',
  },
];

export const mockProjects: ProjectSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000020',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    slug: 'agent-workspace',
    name: 'agent-workspace',
    status: 'active',
  },
];

export const mockTaskGroups: TaskGroupSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000030',
    project_id: '00000000-0000-0000-0000-000000000020',
    kind: 'initiative',
    title: 'Domain foundation',
    status: 'active',
    priority: 100,
  },
];

export const mockTasks: TaskDetail[] = [
  {
    id: '00000000-0000-0000-0000-000000000040',
    project_id: '00000000-0000-0000-0000-000000000020',
    group_id: '00000000-0000-0000-0000-000000000030',
    parent_task_id: null,
    title: 'Frontend API foundation',
    description_md: 'Typed client, query/mutation hooks, dev mocks.',
    status: 'in_progress',
    priority: 'high',
    rank_key: 'a0',
    starts_at: null,
    due_at: null,
    assignee_type: 'workspace_member',
    assignee_id: '00000000-0000-0000-0000-000000000001',
    blocked: false,
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-03-11T10:00:00Z',
  },
  {
    id: '00000000-0000-0000-0000-000000000041',
    project_id: '00000000-0000-0000-0000-000000000020',
    group_id: '00000000-0000-0000-0000-000000000030',
    parent_task_id: null,
    title: 'Shared HTTP runtime',
    description_md: 'request_id, error envelope, pagination primitives.',
    status: 'todo',
    priority: 'high',
    rank_key: 'a1',
    starts_at: null,
    due_at: null,
    assignee_type: null,
    assignee_id: null,
    blocked: false,
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-03-11T10:00:00Z',
  },
];

export const mockNotes: NoteDetail[] = [
  {
    id: '00000000-0000-0000-0000-000000000050',
    project_id: '00000000-0000-0000-0000-000000000020',
    agent_session_id: null,
    kind: 'decision',
    author_type: 'workspace_member',
    author_id: '00000000-0000-0000-0000-000000000001',
    title: 'SQLite profile is local-only',
    body_md: 'SQLite used for local/dev profile only. Postgres is the production target.',
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-03-11T10:00:00Z',
  },
];
