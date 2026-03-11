import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import {
  createTask,
  createTaskGroup,
  getTask,
  getTaskGroup,
  listTaskGroups,
  listTasks,
  patchTaskGroup,
  updateTaskStatus,
  type ListTasksParams,
} from '../api/tasks';
import type {
  CreateTaskGroupPayload,
  CreateTaskPayload,
  PatchTaskGroupPayload,
  UpdateTaskStatusPayload,
} from '../api/types';

// ─── Task groups ──────────────────────────────────────────────────────────────

export function useTaskGroups(workspaceSlug: string, projectSlug: string) {
  return useQuery({
    queryKey: queryKeys.taskGroups(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listTaskGroups(workspaceSlug, projectSlug, undefined, { signal }),
  });
}

export function useTaskGroup(workspaceSlug: string, projectSlug: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.taskGroup(workspaceSlug, projectSlug, groupId),
    queryFn: ({ signal }) => getTaskGroup(workspaceSlug, projectSlug, groupId, { signal }),
  });
}

export function useCreateTaskGroup(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateTaskGroupPayload) =>
      createTaskGroup(workspaceSlug, projectSlug, payload),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.taskGroups(workspaceSlug, projectSlug),
      });
    },
  });
}

export function usePatchTaskGroup(workspaceSlug: string, projectSlug: string, groupId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: PatchTaskGroupPayload) =>
      patchTaskGroup(workspaceSlug, projectSlug, groupId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.taskGroup(workspaceSlug, projectSlug, groupId), updated);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.taskGroups(workspaceSlug, projectSlug),
      });
    },
  });
}

// ─── Tasks ────────────────────────────────────────────────────────────────────

export function useTasks(workspaceSlug: string, projectSlug: string, params?: ListTasksParams) {
  return useQuery({
    queryKey: queryKeys.tasks(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listTasks(workspaceSlug, projectSlug, params, { signal }),
  });
}

export function useTask(workspaceSlug: string, projectSlug: string, taskId: string) {
  return useQuery({
    queryKey: queryKeys.task(workspaceSlug, projectSlug, taskId),
    queryFn: ({ signal }) => getTask(workspaceSlug, projectSlug, taskId, { signal }),
  });
}

export function useCreateTask(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateTaskPayload) =>
      createTask(workspaceSlug, projectSlug, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(
        queryKeys.task(workspaceSlug, projectSlug, created.id),
        created,
      );
      void queryClient.invalidateQueries({
        queryKey: queryKeys.tasks(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useUpdateTaskStatus(workspaceSlug: string, projectSlug: string, taskId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateTaskStatusPayload) =>
      updateTaskStatus(workspaceSlug, projectSlug, taskId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(
        queryKeys.task(workspaceSlug, projectSlug, taskId),
        updated,
      );
      void queryClient.invalidateQueries({
        queryKey: queryKeys.tasks(workspaceSlug, projectSlug),
      });
    },
  });
}
