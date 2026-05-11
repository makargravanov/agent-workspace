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
  const resp = await apiPost<CreateAgentCredentialPayload, ApiResponse<CreatedAgentCredential>>(
    `/workspaces/${workspaceSlug}/agents/${agentId}/credentials`,
    payload,
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
  const resp = await apiPatch<UpdateAgentCredentialPayload, ApiResponse<AgentCredentialSummary>>(
    `/workspaces/${workspaceSlug}/agent-credentials/${credentialId}`,
    payload,
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
