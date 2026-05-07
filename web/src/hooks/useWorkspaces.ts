import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import {
  createProject,
  createWorkspace,
  getProject,
  getWorkspace,
  listProjects,
  listWorkspaces,
} from '../api/workspaces';
import type { CreateProjectPayload, CreateWorkspacePayload, PaginationParams } from '../api/types';

export function useWorkspaces(pagination?: PaginationParams, enabled = true) {
  return useQuery({
    queryKey: queryKeys.workspaces(),
    queryFn: ({ signal }) => listWorkspaces(pagination, { signal }),
    enabled,
  });
}

export function useWorkspace(workspaceSlug: string) {
  return useQuery({
    queryKey: queryKeys.workspace(workspaceSlug),
    queryFn: ({ signal }) => getWorkspace(workspaceSlug, { signal }),
    enabled: workspaceSlug.length > 0,
  });
}

export function useProjects(workspaceSlug: string, pagination?: PaginationParams) {
  return useQuery({
    queryKey: queryKeys.projects(workspaceSlug),
    queryFn: ({ signal }) => listProjects(workspaceSlug, pagination, { signal }),
    enabled: workspaceSlug.length > 0,
  });
}

export function useProject(workspaceSlug: string, projectSlug: string) {
  return useQuery({
    queryKey: queryKeys.project(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => getProject(workspaceSlug, projectSlug, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
  });
}

export function useCreateWorkspace() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateWorkspacePayload) => createWorkspace(payload),
    onSuccess: (created) => {
      queryClient.setQueryData(queryKeys.workspace(created.slug), created);
      void queryClient.invalidateQueries({ queryKey: queryKeys.workspaces() });
    },
  });
}

export function useCreateProject(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateProjectPayload) => createProject(workspaceSlug, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(queryKeys.project(workspaceSlug, created.slug), created);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.projects(workspaceSlug),
      });
    },
  });
}
