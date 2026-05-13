import { apiGet, type RequestOptions } from './client';
import type { ActivityEvent, ApiListResponse, PaginationParams } from './types';

export async function listWorkspaceActivity(
  workspaceSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<ActivityEvent>['data']> {
  const resp = await apiGet<ApiListResponse<ActivityEvent>>(
    `/workspaces/${workspaceSlug}/activity`,
    pagination,
    opts,
  );
  return resp.data;
}

export async function listProjectActivity(
  workspaceSlug: string,
  projectSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<ActivityEvent>['data']> {
  const resp = await apiGet<ApiListResponse<ActivityEvent>>(
    `/workspaces/${workspaceSlug}/projects/${projectSlug}/activity`,
    pagination,
    opts,
  );
  return resp.data;
}
