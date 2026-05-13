export interface ApiMeta {
  request_id: string;
  audit_event_id?: string;
}

export interface ApiResponse<T> {
  data: T;
  meta: ApiMeta;
}

export interface ApiListData<T> {
  items: T[];
  next_cursor: string | null;
}

export interface ApiListResponse<T> {
  data: ApiListData<T>;
  meta: ApiMeta;
}

export interface ApiErrorBody {
  code: string;
  message: string;
  details: unknown | null;
  request_id: string;
}

export interface PaginationParams {
  cursor?: string;
  limit?: number;
  [key: string]: string | number | undefined;
}

export type ActorKind = 'human' | 'agent' | 'system';
export type WorkspaceRole = 'owner' | 'editor' | 'viewer' | 'member';

export interface ActorContext {
  actor_kind: ActorKind;
  actor_id: string;
  workspace_id?: string;
  project_id?: string;
  role?: WorkspaceRole | string;
  scopes: string[];
}

export interface Session {
  authenticated: boolean;
  actor?: ActorContext;
}

export interface WorkspaceSummary {
  id: string;
  slug: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export type ProjectStatus = 'active' | 'archived' | string;

export interface ProjectSummary {
  id: string;
  workspace_id: string;
  slug: string;
  name: string;
  status: ProjectStatus;
  created_at: string;
  updated_at: string;
}

export type TaskGroupKind = 'initiative' | 'milestone' | 'sprint';
export type TaskGroupStatus = 'active' | 'completed' | 'archived';

export interface TaskGroupSummary {
  id: string;
  project_id: string;
  kind: TaskGroupKind;
  title: string;
  status: TaskGroupStatus;
  priority: number;
}

export type TaskStatus = 'todo' | 'in_progress' | 'done' | 'cancelled';
export type TaskPriority = 'low' | 'normal' | 'high' | 'critical';
export type AssigneeType = 'workspace_member' | 'agent';

export interface TaskDetail {
  id: string;
  project_id: string;
  group_id: string | null;
  parent_task_id: string | null;
  title: string;
  description_md: string | null;
  status: TaskStatus;
  priority: TaskPriority;
  rank_key: string;
  starts_at: string | null;
  due_at: string | null;
  assignee_type: AssigneeType | null;
  assignee_id: string | null;
  blocked: boolean;
  created_at: string;
  updated_at: string;
}

export interface TaskDependency {
  id: string;
  task_id: string;
  depends_on_task_id: string;
}

export type NoteKind = 'context' | 'worklog' | 'decision' | 'result';
export type AuthorType = 'workspace_member' | 'agent' | 'integration';

export interface NoteDetail {
  id: string;
  project_id: string;
  agent_session_id: string | null;
  kind: NoteKind;
  author_type: AuthorType;
  author_id: string;
  title: string | null;
  body_md: string;
  created_at: string;
  updated_at: string;
}

export type DocumentStatus = 'draft' | 'published' | 'archived';

export interface DocumentDetail {
  id: string;
  workspace_id: string;
  project_id: string;
  parent_document_id: string | null;
  slug: string;
  title: string;
  body_format: string;
  body_md: string;
  status: DocumentStatus;
  version: number;
  created_at: string;
  updated_at: string;
}

export interface AssetDetail {
  id: string;
  workspace_id: string;
  project_id: string;
  uploaded_by_member_id: string | null;
  uploaded_by_github_login: string | null;
  file_name: string;
  media_type: string;
  size_bytes: number;
  sha256: string | null;
  storage_backend: string;
  storage_key: string;
  created_at: string;
}

export interface CreateTaskPayload {
  group_id?: string | null;
  parent_task_id?: string | null;
  title: string;
  description_md?: string | null;
  priority?: TaskPriority;
  rank_key?: string;
  assignee_type?: AssigneeType | null;
  assignee_id?: string | null;
}

export interface UpdateTaskStatusPayload {
  status: TaskStatus;
}

export interface CreateTaskGroupPayload {
  kind: TaskGroupKind;
  title: string;
  priority?: number;
}

export interface PatchTaskGroupPayload {
  title?: string;
  status?: TaskGroupStatus;
  priority?: number;
}

export interface CreateNotePayload {
  kind: NoteKind;
  title?: string | null;
  body_md: string;
  agent_session_id?: string | null;
}

export interface CreateDocumentPayload {
  slug: string;
  title: string;
  body_md: string;
  parent_document_id?: string | null;
  body_format?: string;
  status?: DocumentStatus;
}

export interface UpdateDocumentPayload {
  version: number;
  slug?: string;
  title?: string;
  body_md?: string;
  parent_document_id?: string | null;
  body_format?: string;
  status?: DocumentStatus;
}

export interface CreateAssetPayload {
  file_name: string;
  media_type: string;
  content_base64: string;
  sha256?: string | null;
}

export interface UpdateAssetPayload {
  file_name?: string;
  media_type?: string;
  content_base64?: string;
  sha256?: string | null;
}

export interface RepairDocumentCyclesResult {
  repaired_document_ids: string[];
  cycle_groups: string[][];
}

export interface CreateWorkspacePayload {
  slug: string;
  name: string;
}

export interface CreateProjectPayload {
  slug: string;
  name: string;
}

export type AgentStatus = 'active' | 'disabled';

export interface AgentSummary {
  id: string;
  workspace_id: string;
  key: string;
  display_name: string;
  status: AgentStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateAgentPayload {
  key: string;
  display_name: string;
}

export interface UpdateAgentPayload {
  key?: string;
  display_name?: string;
  status?: AgentStatus;
}

export type CredentialStatus = 'active' | 'revoked';

export interface AgentCredentialSummary {
  id: string;
  workspace_id: string;
  project_id: string | null;
  agent_id: string;
  label: string;
  secret_prefix: string;
  scope_policy: string[] | string;
  status: CredentialStatus;
  expires_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateAgentCredentialPayload {
  label: string;
  project_id?: string | null;
  scopes: string[];
  expires_at?: string | null;
}

export interface UpdateAgentCredentialPayload {
  label?: string;
  project_id?: string | null;
  scopes?: string[];
  status?: CredentialStatus;
  expires_at?: string | null;
}

export interface CreatedAgentCredential {
  credential: AgentCredentialSummary;
  secret: string;
}

export type IntegrationProvider = 'github';
export type IntegrationScopeKind = 'workspace' | 'project';
export type IntegrationConnectionStatus = 'active' | 'disabled' | 'error';

export interface IntegrationConnectionSummary {
  id: string;
  workspace_id: string;
  project_id: string | null;
  provider: IntegrationProvider | string;
  scope_kind: IntegrationScopeKind | string;
  status: IntegrationConnectionStatus | string;
  config_json: string | null;
  secret_ciphertext: string | null;
  last_synced_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateIntegrationConnectionPayload {
  provider: IntegrationProvider;
  scope_kind: IntegrationScopeKind;
  project_id?: string | null;
  status?: IntegrationConnectionStatus;
  config_json?: unknown;
}

export interface UpdateIntegrationConnectionPayload {
  status?: IntegrationConnectionStatus;
  config_json?: unknown;
}

export interface ActivityEvent {
  id: string;
  workspace_id: string;
  project_id: string | null;
  actor_type: string;
  actor_id: string | null;
  actor_github_login?: string | null;
  entity_type: string;
  entity_id: string | null;
  event_type: string;
  payload_json: string | null;
  occurred_at: string;
}

export type SearchResultKind =
  | 'workspace'
  | 'project'
  | 'task'
  | 'task_group'
  | 'note'
  | 'document'
  | 'asset'
  | 'agent'
  | 'integration_connection'
  | string;

export interface SearchResult {
  kind: SearchResultKind;
  id: string;
  workspace_id: string | null;
  project_id: string | null;
  title: string;
  summary: string | null;
  updated_at: string;
}

export interface SearchParams {
  q: string;
  workspace_slug?: string;
  project_slug?: string;
}
