import { useQuery } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import { getProject, getWorkspace, listProjects, listWorkspaces } from '../api/workspaces';
import type { PaginationParams } from '../api/types';

export function useWorkspaces(pagination?: PaginationParams) {
  return useQuery({
    queryKey: queryKeys.workspaces(),
    queryFn: ({ signal }) => listWorkspaces(pagination, { signal }),
  });
}

export function useWorkspace(workspaceSlug: string) {
  return useQuery({
    queryKey: queryKeys.workspace(workspaceSlug),
    queryFn: ({ signal }) => getWorkspace(workspaceSlug, { signal }),
  });
}

export function useProjects(workspaceSlug: string, pagination?: PaginationParams) {
  return useQuery({
    queryKey: queryKeys.projects(workspaceSlug),
    queryFn: ({ signal }) => listProjects(workspaceSlug, pagination, { signal }),
  });
}

export function useProject(workspaceSlug: string, projectSlug: string) {
  return useQuery({
    queryKey: queryKeys.project(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => getProject(workspaceSlug, projectSlug, { signal }),
  });
}
