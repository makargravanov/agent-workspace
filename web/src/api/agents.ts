import { apiDelete, apiGet, apiPatch, apiPost, type RequestOptions } from './client';
import type {
  AgentCredentialSummary,
  AgentSummary,
  ApiListResponse,
  ApiResponse,
  CreateAgentCredentialPayload,
  CreateAgentPayload,
  CreatedAgentCredential,
  PaginationParams,
  UpdateAgentCredentialPayload,
  UpdateAgentPayload,
} from './types';

type CreateAgentCredentialRequest = {
  label: string;
  project_id: string | null;
  scope_policy: string[];
  expires_at: string | null;
};

type UpdateAgentCredentialRequest = {
  label?: string;
  project_id?: string | null;
  scope_policy?: string[];
  status?: 'active' | 'revoked';
  expires_at?: string | null;
};

export async function listAgents(
  workspaceSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<AgentSummary>['data']> {
  const resp = await apiGet<ApiListResponse<AgentSummary>>(
    `/workspaces/${workspaceSlug}/agents`,
    pagination,
    opts,
  );
  return resp.data;
}

export async function getAgent(
  workspaceSlug: string,
  agentId: string,
  opts?: RequestOptions,
): Promise<AgentSummary> {
  const resp = await apiGet<ApiResponse<AgentSummary>>(
    `/workspaces/${workspaceSlug}/agents/${agentId}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createAgent(
  workspaceSlug: string,
  payload: CreateAgentPayload,
  opts?: RequestOptions,
): Promise<AgentSummary> {
  const resp = await apiPost<CreateAgentPayload, ApiResponse<AgentSummary>>(
    `/workspaces/${workspaceSlug}/agents`,
    payload,
    opts,
  );
  return resp.data;
}

export async function updateAgent(
  workspaceSlug: string,
  agentId: string,
  payload: UpdateAgentPayload,
  opts?: RequestOptions,
): Promise<AgentSummary> {
  const resp = await apiPatch<UpdateAgentPayload, ApiResponse<AgentSummary>>(
    `/workspaces/${workspaceSlug}/agents/${agentId}`,
    payload,
    opts,
  );
  return resp.data;
}

export async function deleteAgent(
  workspaceSlug: string,
  agentId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(`/workspaces/${workspaceSlug}/agents/${agentId}`, opts);
}

export async function listAgentCredentials(
  workspaceSlug: string,
  agentId: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<AgentCredentialSummary>['data']> {
  const resp = await apiGet<ApiListResponse<AgentCredentialSummary>>(
    `/workspaces/${workspaceSlug}/agents/${agentId}/credentials`,
    pagination,
    opts,
  );
  return resp.data;
}

export async function createAgentCredential(
  workspaceSlug: string,
  agentId: string,
  payload: CreateAgentCredentialPayload,
  opts?: RequestOptions,
): Promise<CreatedAgentCredential> {
  const body = {
    label: payload.label,
    project_id: payload.project_id ?? null,
    scope_policy: payload.scopes,
    expires_at: payload.expires_at ?? null,
  };
  const resp = await apiPost<CreateAgentCredentialRequest, ApiResponse<CreatedAgentCredential>>(
    `/workspaces/${workspaceSlug}/agents/${agentId}/credentials`,
    body,
    opts,
  );
  return resp.data;
}

export async function getAgentCredential(
  workspaceSlug: string,
  credentialId: string,
  opts?: RequestOptions,
): Promise<AgentCredentialSummary> {
  const resp = await apiGet<ApiResponse<AgentCredentialSummary>>(
    `/workspaces/${workspaceSlug}/agent-credentials/${credentialId}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function updateAgentCredential(
  workspaceSlug: string,
  credentialId: string,
  payload: UpdateAgentCredentialPayload,
  opts?: RequestOptions,
): Promise<AgentCredentialSummary> {
  const body = {
    label: payload.label,
    project_id: payload.project_id ?? null,
    scope_policy: payload.scopes,
    status: payload.status,
    expires_at: payload.expires_at ?? null,
  };
  const resp = await apiPatch<UpdateAgentCredentialRequest, ApiResponse<AgentCredentialSummary>>(
    `/workspaces/${workspaceSlug}/agent-credentials/${credentialId}`,
    body,
    opts,
  );
  return resp.data;
}

export async function deleteAgentCredential(
  workspaceSlug: string,
  credentialId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(`/workspaces/${workspaceSlug}/agent-credentials/${credentialId}`, opts);
}
