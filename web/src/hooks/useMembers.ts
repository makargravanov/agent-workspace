import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  createWorkspaceInvite,
  deleteWorkspaceInvite,
  deleteProjectMember,
  listProjectMembers,
  listWorkspaceInvites,
  listWorkspaceMembers,
  updateWorkspaceMember,
  upsertProjectMember,
} from '../api/members';
import { queryKeys } from '../api/query-keys';
import type {
  CreateWorkspaceInvitePayload,
  UpdateWorkspaceMemberPayload,
  UpsertProjectMemberPayload,
} from '../api/types';

export function useWorkspaceMembers(workspaceSlug: string) {
  return useQuery({
    queryKey: queryKeys.members(workspaceSlug),
    queryFn: ({ signal }) => listWorkspaceMembers(workspaceSlug, { signal }),
    enabled: workspaceSlug.length > 0,
  });
}

export function useWorkspaceInvites(workspaceSlug: string) {
  return useQuery({
    queryKey: queryKeys.invites(workspaceSlug),
    queryFn: ({ signal }) => listWorkspaceInvites(workspaceSlug, { signal }),
    enabled: workspaceSlug.length > 0,
  });
}

export function useCreateWorkspaceInvite(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateWorkspaceInvitePayload) => createWorkspaceInvite(workspaceSlug, payload),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.invites(workspaceSlug) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.members(workspaceSlug) });
    },
  });
}

export function useDeleteWorkspaceInvite(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (inviteId: string) => deleteWorkspaceInvite(workspaceSlug, inviteId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.invites(workspaceSlug) });
    },
  });
}

export function useUpdateWorkspaceMember(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ memberId, payload }: { memberId: string; payload: UpdateWorkspaceMemberPayload }) =>
      updateWorkspaceMember(workspaceSlug, memberId, payload),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.members(workspaceSlug) });
    },
  });
}

export function useProjectMembers(workspaceSlug: string, projectSlug: string) {
  return useQuery({
    queryKey: queryKeys.projectMembers(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listProjectMembers(workspaceSlug, projectSlug, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
  });
}

export function useUpsertProjectMember(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ memberId, payload }: { memberId: string; payload: UpsertProjectMemberPayload }) =>
      upsertProjectMember(workspaceSlug, projectSlug, memberId, payload),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.projectMembers(workspaceSlug, projectSlug) });
    },
  });
}

export function useDeleteProjectMember(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (memberId: string) => deleteProjectMember(workspaceSlug, projectSlug, memberId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.projectMembers(workspaceSlug, projectSlug) });
    },
  });
}
