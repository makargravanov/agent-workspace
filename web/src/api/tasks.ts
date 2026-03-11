import { apiDelete, apiGet, apiPatch, apiPost, type RequestOptions } from './client';
import type {
  ApiListResponse,
  ApiResponse,
  CreateTaskGroupPayload,
  CreateTaskPayload,
  PaginationParams,
  PatchTaskGroupPayload,
  TaskDependency,
  TaskDetail,
  TaskGroupSummary,
  TaskStatus,
  UpdateTaskStatusPayload,
} from './types';

// ─── Task groups ──────────────────────────────────────────────────────────────

const groupsBase = (ws: string, proj: string) =>
  `/workspaces/${ws}/projects/${proj}/task-groups`;

export async function listTaskGroups(
  workspaceSlug: string,
  projectSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<TaskGroupSummary>['data']> {
  const resp = await apiGet<ApiListResponse<TaskGroupSummary>>(
    groupsBase(workspaceSlug, projectSlug),
    pagination,
    opts,
  );
  return resp.data;
}

export async function getTaskGroup(
  workspaceSlug: string,
  projectSlug: string,
  groupId: string,
  opts?: RequestOptions,
): Promise<TaskGroupSummary> {
  const resp = await apiGet<ApiResponse<TaskGroupSummary>>(
    `${groupsBase(workspaceSlug, projectSlug)}/${groupId}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createTaskGroup(
  workspaceSlug: string,
  projectSlug: string,
  payload: CreateTaskGroupPayload,
  opts?: RequestOptions,
): Promise<TaskGroupSummary> {
  const resp = await apiPost<CreateTaskGroupPayload, ApiResponse<TaskGroupSummary>>(
    groupsBase(workspaceSlug, projectSlug),
    payload,
    opts,
  );
  return resp.data;
}

export async function patchTaskGroup(
  workspaceSlug: string,
  projectSlug: string,
  groupId: string,
  payload: PatchTaskGroupPayload,
  opts?: RequestOptions,
): Promise<TaskGroupSummary> {
  const resp = await apiPatch<PatchTaskGroupPayload, ApiResponse<TaskGroupSummary>>(
    `${groupsBase(workspaceSlug, projectSlug)}/${groupId}`,
    payload,
    opts,
  );
  return resp.data;
}

// ─── Tasks ────────────────────────────────────────────────────────────────────

const tasksBase = (ws: string, proj: string) =>
  `/workspaces/${ws}/projects/${proj}/tasks`;

export interface ListTasksParams extends PaginationParams {
  status?: TaskStatus;
  group_id?: string;
  assignee_id?: string;
}

export async function listTasks(
  workspaceSlug: string,
  projectSlug: string,
  params?: ListTasksParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<TaskDetail>['data']> {
  const resp = await apiGet<ApiListResponse<TaskDetail>>(
    tasksBase(workspaceSlug, projectSlug),
    params,
    opts,
  );
  return resp.data;
}

export async function getTask(
  workspaceSlug: string,
  projectSlug: string,
  taskId: string,
  opts?: RequestOptions,
): Promise<TaskDetail> {
  const resp = await apiGet<ApiResponse<TaskDetail>>(
    `${tasksBase(workspaceSlug, projectSlug)}/${taskId}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createTask(
  workspaceSlug: string,
  projectSlug: string,
  payload: CreateTaskPayload,
  opts?: RequestOptions,
): Promise<TaskDetail> {
  const resp = await apiPost<CreateTaskPayload, ApiResponse<TaskDetail>>(
    tasksBase(workspaceSlug, projectSlug),
    payload,
    opts,
  );
  return resp.data;
}

export async function updateTaskStatus(
  workspaceSlug: string,
  projectSlug: string,
  taskId: string,
  payload: UpdateTaskStatusPayload,
  opts?: RequestOptions,
): Promise<TaskDetail> {
  const resp = await apiPatch<UpdateTaskStatusPayload, ApiResponse<TaskDetail>>(
    `${tasksBase(workspaceSlug, projectSlug)}/${taskId}/status`,
    payload,
    opts,
  );
  return resp.data;
}

// ─── Task dependencies ────────────────────────────────────────────────────────

export async function listTaskDependencies(
  workspaceSlug: string,
  projectSlug: string,
  taskId: string,
  opts?: RequestOptions,
): Promise<ApiListResponse<TaskDependency>['data']> {
  const resp = await apiGet<ApiListResponse<TaskDependency>>(
    `${tasksBase(workspaceSlug, projectSlug)}/${taskId}/dependencies`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createTaskDependency(
  workspaceSlug: string,
  projectSlug: string,
  taskId: string,
  dependsOnTaskId: string,
  opts?: RequestOptions,
): Promise<TaskDependency> {
  const resp = await apiPost<{ depends_on_task_id: string }, ApiResponse<TaskDependency>>(
    `${tasksBase(workspaceSlug, projectSlug)}/${taskId}/dependencies`,
    { depends_on_task_id: dependsOnTaskId },
    opts,
  );
  return resp.data;
}

export async function deleteTaskDependency(
  workspaceSlug: string,
  projectSlug: string,
  taskId: string,
  dependencyId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(
    `${tasksBase(workspaceSlug, projectSlug)}/${taskId}/dependencies/${dependencyId}`,
    opts,
  );
}
