import { apiGet, apiPost, type RequestOptions } from './client';
import type {
  ApiListResponse,
  ApiResponse,
  CreateProjectPayload,
  CreateWorkspacePayload,
  PaginationParams,
  ProjectSummary,
  WorkspaceSummary,
} from './types';

// ─── Workspaces ───────────────────────────────────────────────────────────────

export async function listWorkspaces(
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<WorkspaceSummary>['data']> {
  const resp = await apiGet<ApiListResponse<WorkspaceSummary>>(
    '/workspaces',
    pagination,
    opts,
  );
  return resp.data;
}

export async function getWorkspace(
  workspaceSlug: string,
  opts?: RequestOptions,
): Promise<WorkspaceSummary> {
  const resp = await apiGet<ApiResponse<WorkspaceSummary>>(
    `/workspaces/${workspaceSlug}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createWorkspace(
  payload: CreateWorkspacePayload,
  opts?: RequestOptions,
): Promise<WorkspaceSummary> {
  const resp = await apiPost<CreateWorkspacePayload, ApiResponse<WorkspaceSummary>>(
    '/workspaces',
    payload,
    opts,
  );
  return resp.data;
}

// ─── Projects ─────────────────────────────────────────────────────────────────

export async function listProjects(
  workspaceSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<ProjectSummary>['data']> {
  const resp = await apiGet<ApiListResponse<ProjectSummary>>(
    `/workspaces/${workspaceSlug}/projects`,
    pagination,
    opts,
  );
  return resp.data;
}

export async function getProject(
  workspaceSlug: string,
  projectSlug: string,
  opts?: RequestOptions,
): Promise<ProjectSummary> {
  const resp = await apiGet<ApiResponse<ProjectSummary>>(
    `/workspaces/${workspaceSlug}/projects/${projectSlug}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createProject(
  workspaceSlug: string,
  payload: CreateProjectPayload,
  opts?: RequestOptions,
): Promise<ProjectSummary> {
  const resp = await apiPost<CreateProjectPayload, ApiResponse<ProjectSummary>>(
    `/workspaces/${workspaceSlug}/projects`,
    payload,
    opts,
  );
  return resp.data;
}
