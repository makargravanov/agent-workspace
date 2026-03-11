// ─── API envelope types ───────────────────────────────────────────────────────

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

// ─── Pagination ───────────────────────────────────────────────────────────────

export interface PaginationParams {
  cursor?: string;
  limit?: number;
  // Index signature so subtypes are accepted as Record<string, ...> in apiGet params.
  [key: string]: string | number | undefined;
}

// ─── Auth / Session ───────────────────────────────────────────────────────────

export interface SessionUser {
  id: string;
  name: string;
  email: string;
  avatar_url: string | null;
}

export interface Session {
  user: SessionUser;
  workspace_id: string;
}

// ─── Workspace / Project ──────────────────────────────────────────────────────

export interface WorkspaceSummary {
  id: string;
  slug: string;
  name: string;
}

export type ProjectStatus = 'active' | 'archived';

export interface ProjectSummary {
  id: string;
  workspace_id: string;
  slug: string;
  name: string;
  status: ProjectStatus;
}

// ─── Task groups ──────────────────────────────────────────────────────────────

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

// ─── Tasks ────────────────────────────────────────────────────────────────────

export type TaskStatus = 'todo' | 'in_progress' | 'done' | 'cancelled';
export type TaskPriority = 'low' | 'medium' | 'high' | 'critical';
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

// ─── Notes ───────────────────────────────────────────────────────────────────

export type NoteKind = 'decision' | 'observation' | 'context';
export type AuthorType = 'workspace_member' | 'agent';

export interface NoteDetail {
  id: string;
  project_id: string;
  agent_session_id: string | null;
  kind: NoteKind;
  author_type: AuthorType;
  author_id: string;
  title: string;
  body_md: string;
  created_at: string;
  updated_at: string;
}

// ─── Documents ───────────────────────────────────────────────────────────────

export type DocumentStatus = 'draft' | 'published' | 'archived';

export interface DocumentDetail {
  id: string;
  project_id: string;
  parent_document_id: string | null;
  slug: string;
  title: string;
  body_format: 'markdown';
  body_md: string;
  status: DocumentStatus;
  version: number;
  created_at: string;
  updated_at: string;
}

// ─── Members ─────────────────────────────────────────────────────────────────

export type MemberRole = 'owner' | 'member';
export type MemberStatus = 'active' | 'invited' | 'disabled';

export interface WorkspaceMember {
  id: string;
  workspace_id: string;
  user_id: string;
  role: MemberRole;
  status: MemberStatus;
  joined_at: string | null;
}

// ─── Agents / Credentials ────────────────────────────────────────────────────

export type AgentStatus = 'active' | 'disabled';

export interface AgentSummary {
  id: string;
  workspace_id: string;
  name: string;
  status: AgentStatus;
  created_at: string;
}

export type AgentScope =
  | 'tasks:read'
  | 'tasks:write_status'
  | 'task_groups:read'
  | 'documents:read'
  | 'assets:read'
  | 'notes:read'
  | 'notes:write'
  | 'audit:read_recent';

export interface CredentialMeta {
  id: string;
  agent_id: string;
  label: string;
  project_id: string | null;
  scopes: AgentScope[];
  expires_at: string | null;
  created_at: string;
  revoked_at: string | null;
}

// ─── Mutation payloads ───────────────────────────────────────────────────────

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
  title: string;
  body_md: string;
  agent_session_id?: string | null;
}

export interface CreateWorkspacePayload {
  slug: string;
  name: string;
}

export interface CreateProjectPayload {
  slug: string;
  name: string;
}
