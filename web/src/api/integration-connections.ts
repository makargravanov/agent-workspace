import { apiDelete, apiGet, apiPatch, apiPost, type RequestOptions } from './client';
import type {
  ApiListResponse,
  ApiResponse,
  CreateIntegrationConnectionPayload,
  IntegrationConnectionSummary,
  PaginationParams,
  UpdateIntegrationConnectionPayload,
} from './types';

export async function listIntegrationConnections(
  workspaceSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<IntegrationConnectionSummary>['data']> {
  const resp = await apiGet<ApiListResponse<IntegrationConnectionSummary>>(
    `/workspaces/${workspaceSlug}/integration-connections`,
    pagination,
    opts,
  );
  return resp.data;
}

export async function createIntegrationConnection(
  workspaceSlug: string,
  payload: CreateIntegrationConnectionPayload,
  opts?: RequestOptions,
): Promise<IntegrationConnectionSummary> {
  const body = {
    provider: payload.provider,
    scope_kind: payload.scope_kind,
    project_id: payload.scope_kind === 'project' ? payload.project_id ?? null : null,
    status: payload.status ?? 'active',
    config_json: payload.config_json ?? null,
  };
  const resp = await apiPost<typeof body, ApiResponse<IntegrationConnectionSummary>>(
    `/workspaces/${workspaceSlug}/integration-connections`,
    body,
    opts,
  );
  return resp.data;
}

export async function updateIntegrationConnection(
  workspaceSlug: string,
  connectionId: string,
  payload: UpdateIntegrationConnectionPayload,
  opts?: RequestOptions,
): Promise<IntegrationConnectionSummary> {
  const resp = await apiPatch<UpdateIntegrationConnectionPayload, ApiResponse<IntegrationConnectionSummary>>(
    `/workspaces/${workspaceSlug}/integration-connections/${connectionId}`,
    payload,
    opts,
  );
  return resp.data;
}

export async function deleteIntegrationConnection(
  workspaceSlug: string,
  connectionId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(`/workspaces/${workspaceSlug}/integration-connections/${connectionId}`, opts);
}
