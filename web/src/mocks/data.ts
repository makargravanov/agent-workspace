import type {
  AgentCredentialSummary,
  AgentSummary,
  ActivityEvent,
  AssetDetail,
  DocumentDetail,
  IntegrationConnectionSummary,
  NoteDetail,
  ProjectSummary,
  Session,
  TaskDetail,
  WorkspaceSummary,
} from '../api/types';

export const mockSession: Session = {
  authenticated: true,
  actor: {
    actor_kind: 'human',
    actor_id: '00000000-0000-0000-0000-000000000001',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    role: 'owner',
    scopes: [],
  },
};

export const mockWorkspaces: WorkspaceSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000010',
    slug: 'core-platform',
    name: 'Core Platform',
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-03-11T10:00:00Z',
  },
];

export const mockProjects: ProjectSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000020',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    slug: 'agent-workspace',
    name: 'agent-workspace',
    status: 'active',
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-03-11T10:00:00Z',
  },
];

export const mockAgents: AgentSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000030',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    key: 'automation-bot',
    display_name: 'Automation Bot',
    status: 'active',
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-03-11T10:00:00Z',
  },
];

export const mockAgentCredentials: AgentCredentialSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000031',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: null,
    agent_id: '00000000-0000-0000-0000-000000000030',
    label: 'cli-local',
    secret_prefix: 'awsk_',
    scope_policy: ['tasks:read', 'notes:write'],
    status: 'active',
    expires_at: null,
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-03-11T10:00:00Z',
  },
];

export const mockIntegrationConnections: IntegrationConnectionSummary[] = [
  {
    id: '00000000-0000-0000-0000-000000000070',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: null,
    provider: 'github',
    scope_kind: 'workspace',
    status: 'active',
    config_json: '{"repo":"openai/agent-workspace","events":["issues","pull_requests"]}',
    secret_ciphertext: null,
    last_synced_at: null,
    created_at: '2026-05-01T10:00:00Z',
    updated_at: '2026-05-01T10:00:00Z',
  },
  {
    id: '00000000-0000-0000-0000-000000000071',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: '00000000-0000-0000-0000-000000000020',
    provider: 'github',
    scope_kind: 'project',
    status: 'disabled',
    config_json: '{"repo":"openai/project-board"}',
    secret_ciphertext: null,
    last_synced_at: null,
    created_at: '2026-05-02T10:00:00Z',
    updated_at: '2026-05-02T10:00:00Z',
  },
];

export const mockTasks: TaskDetail[] = [
  {
    id: '00000000-0000-0000-0000-000000000040',
    project_id: '00000000-0000-0000-0000-000000000020',
    group_id: null,
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
    group_id: null,
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

export const mockDocuments: DocumentDetail[] = [
  {
    id: '00000000-0000-0000-0000-000000000060',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: '00000000-0000-0000-0000-000000000020',
    parent_document_id: null,
    slug: 'project-guide',
    title: 'Project Guide',
    body_format: 'markdown',
    body_md: '# Project Guide\n\nThis is the primary project reference document.',
    status: 'published',
    version: 2,
    created_at: '2026-03-11T10:00:00Z',
    updated_at: '2026-05-01T10:00:00Z',
  },
  {
    id: '00000000-0000-0000-0000-000000000061',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: '00000000-0000-0000-0000-000000000020',
    parent_document_id: '00000000-0000-0000-0000-000000000060',
    slug: 'release-notes',
    title: 'Release Notes',
    body_format: 'markdown',
    body_md: '## v1.0\n\n- Initial release',
    status: 'draft',
    version: 1,
    created_at: '2026-04-01T10:00:00Z',
    updated_at: '2026-04-01T10:00:00Z',
  },
];

export const mockAssets: AssetDetail[] = [
  {
    id: '00000000-0000-0000-0000-000000000080',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: '00000000-0000-0000-0000-000000000020',
    uploaded_by_member_id: '00000000-0000-0000-0000-000000000001',
    uploaded_by_github_login: 'makargravanov',
    file_name: 'architecture-note.txt',
    media_type: 'text/plain',
    size_bytes: 42,
    sha256: '7d5b2b4d2f2c6f4f5a6c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6',
    storage_backend: 'local',
    storage_key: '00000000-0000-0000-0000-000000000080',
    created_at: '2026-05-03T10:00:00Z',
  },
];

export const mockActivityEvents: ActivityEvent[] = [
  {
    id: '00000000-0000-0000-0000-000000000091',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: '00000000-0000-0000-0000-000000000020',
    actor_type: 'human',
    actor_id: '00000000-0000-0000-0000-000000000001',
    actor_github_login: 'makargravanov',
    entity_type: 'document',
    entity_id: '00000000-0000-0000-0000-000000000060',
    event_type: 'document.create',
    payload_json: '{"title":"Project Guide"}',
    occurred_at: '2026-05-01T10:00:00Z',
  },
  {
    id: '00000000-0000-0000-0000-000000000092',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: null,
    actor_type: 'human',
    actor_id: '00000000-0000-0000-0000-000000000001',
    actor_github_login: 'makargravanov',
    entity_type: 'agent',
    entity_id: '00000000-0000-0000-0000-000000000030',
    event_type: 'agent.create',
    payload_json: '{"display_name":"Automation Bot"}',
    occurred_at: '2026-05-02T10:00:00Z',
  },
  {
    id: '00000000-0000-0000-0000-000000000093',
    workspace_id: '00000000-0000-0000-0000-000000000010',
    project_id: '00000000-0000-0000-0000-000000000020',
    actor_type: 'integration',
    actor_id: null,
    actor_github_login: null,
    entity_type: 'asset',
    entity_id: '00000000-0000-0000-0000-000000000080',
    event_type: 'asset.create',
    payload_json: '{"file_name":"architecture-note.txt"}',
    occurred_at: '2026-05-03T10:00:00Z',
  },
];
