import { apiDelete, apiGet, apiPatch, apiPost, apiPut, type RequestOptions } from './client';
import type {
  ApiListResponse,
  ApiResponse,
  CreateWorkspaceInvitePayload,
  ProjectMember,
  UpdateWorkspaceMemberPayload,
  UpsertProjectMemberPayload,
  WorkspaceInvite,
  WorkspaceMember,
} from './types';

export async function listWorkspaceMembers(
  workspaceSlug: string,
  opts?: RequestOptions,
): Promise<ApiListResponse<WorkspaceMember>['data']> {
  const resp = await apiGet<ApiListResponse<WorkspaceMember>>(
    `/workspaces/${workspaceSlug}/members`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function listWorkspaceInvites(
  workspaceSlug: string,
  opts?: RequestOptions,
): Promise<ApiListResponse<WorkspaceInvite>['data']> {
  const resp = await apiGet<ApiListResponse<WorkspaceInvite>>(
    `/workspaces/${workspaceSlug}/members/invites`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createWorkspaceInvite(
  workspaceSlug: string,
  payload: CreateWorkspaceInvitePayload,
  opts?: RequestOptions,
): Promise<WorkspaceInvite> {
  const resp = await apiPost<CreateWorkspaceInvitePayload, ApiResponse<WorkspaceInvite>>(
    `/workspaces/${workspaceSlug}/members/invites`,
    payload,
    opts,
  );
  return resp.data;
}

export async function deleteWorkspaceInvite(
  workspaceSlug: string,
  inviteId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(`/workspaces/${workspaceSlug}/members/invites/${inviteId}`, opts);
}

export async function updateWorkspaceMember(
  workspaceSlug: string,
  memberId: string,
  payload: UpdateWorkspaceMemberPayload,
  opts?: RequestOptions,
): Promise<WorkspaceMember> {
  const resp = await apiPatch<UpdateWorkspaceMemberPayload, ApiResponse<WorkspaceMember>>(
    `/workspaces/${workspaceSlug}/members/${memberId}`,
    payload,
    opts,
  );
  return resp.data;
}

export async function listProjectMembers(
  workspaceSlug: string,
  projectSlug: string,
  opts?: RequestOptions,
): Promise<ApiListResponse<ProjectMember>['data']> {
  const resp = await apiGet<ApiListResponse<ProjectMember>>(
    `/workspaces/${workspaceSlug}/projects/${projectSlug}/members`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function upsertProjectMember(
  workspaceSlug: string,
  projectSlug: string,
  memberId: string,
  payload: UpsertProjectMemberPayload,
  opts?: RequestOptions,
): Promise<ProjectMember> {
  const resp = await apiPut<UpsertProjectMemberPayload, ApiResponse<ProjectMember>>(
    `/workspaces/${workspaceSlug}/projects/${projectSlug}/members/${memberId}`,
    payload,
    opts,
  );
  return resp.data;
}

export async function deleteProjectMember(
  workspaceSlug: string,
  projectSlug: string,
  memberId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(`/workspaces/${workspaceSlug}/projects/${projectSlug}/members/${memberId}`, opts);
}
