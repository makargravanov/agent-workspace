import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import {
  deleteProject,
  deleteWorkspace,
  createProject,
  createWorkspace,
  getProject,
  getWorkspace,
  listProjects,
  listWorkspaces,
} from '../api/workspaces';
import type {
  ApiListData,
  CreateProjectPayload,
  CreateWorkspacePayload,
  PaginationParams,
  ProjectSummary,
  WorkspaceSummary,
} from '../api/types';

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

export function useDeleteWorkspace() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (workspaceSlug: string) => deleteWorkspace(workspaceSlug),
    onMutate: async (workspaceSlug) => {
      await queryClient.cancelQueries({ queryKey: queryKeys.workspaces() });

      const previousWorkspaces = queryClient.getQueryData<ApiListData<WorkspaceSummary>>(
        queryKeys.workspaces(),
      );

      queryClient.setQueryData<ApiListData<WorkspaceSummary>>(queryKeys.workspaces(), (current) =>
        current
          ? {
              ...current,
              items: current.items.filter((workspace) => workspace.slug !== workspaceSlug),
            }
          : current,
      );

      return { previousWorkspaces };
    },
    onError: (_error, _workspaceSlug, context) => {
      if (context?.previousWorkspaces) {
        queryClient.setQueryData(queryKeys.workspaces(), context.previousWorkspaces);
      }
    },
    onSuccess: () => {
      void queryClient.removeQueries({
        predicate: (query) =>
          Array.isArray(query.queryKey) && query.queryKey[0] === 'workspaces',
      });
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

export function useDeleteProject(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (projectSlug: string) => deleteProject(workspaceSlug, projectSlug),
    onMutate: async (projectSlug) => {
      const projectsKey = queryKeys.projects(workspaceSlug);

      await queryClient.cancelQueries({ queryKey: projectsKey });

      const previousProjects = queryClient.getQueryData<ApiListData<ProjectSummary>>(projectsKey);

      queryClient.setQueryData<ApiListData<ProjectSummary>>(projectsKey, (current) =>
        current
          ? {
              ...current,
              items: current.items.filter((project) => project.slug !== projectSlug),
            }
          : current,
      );

      return { previousProjects };
    },
    onError: (_error, _projectSlug, context) => {
      if (context?.previousProjects) {
        queryClient.setQueryData(queryKeys.projects(workspaceSlug), context.previousProjects);
      }
    },
    onSuccess: () => {
      void queryClient.removeQueries({
        predicate: (query) =>
          Array.isArray(query.queryKey) &&
          query.queryKey[0] === 'workspaces' &&
          query.queryKey[1] === workspaceSlug,
      });
    },
  });
}
