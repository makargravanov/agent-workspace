import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { queryKeys } from '../api/query-keys';
import {
  deleteTask,
  createTask,
  createTaskGroup,
  getTask,
  getTaskGroup,
  listTaskGroups,
  listTasks,
  patchTaskGroup,
  updateTaskStatus,
  waitForTaskChanges,
  type ListTasksParams,
} from '../api/tasks';
import type {
  ApiListData,
  CreateTaskGroupPayload,
  CreateTaskPayload,
  PatchTaskGroupPayload,
  TaskDetail,
  UpdateTaskStatusPayload,
} from '../api/types';

// ─── Task groups ──────────────────────────────────────────────────────────────

const POLL_RETRY_DELAY_MS = 2_000;

function isAbortError(error: unknown) {
  return error instanceof DOMException && error.name === 'AbortError';
}

function delay(ms: number, signal: AbortSignal) {
  return new Promise<void>((resolve, reject) => {
    if (signal.aborted) {
      reject(new DOMException('Aborted', 'AbortError'));
      return;
    }

    const timeoutId = window.setTimeout(resolve, ms);
    signal.addEventListener(
      'abort',
      () => {
        window.clearTimeout(timeoutId);
        reject(new DOMException('Aborted', 'AbortError'));
      },
      { once: true },
    );
  });
}

export function useTasksLongPolling(
  workspaceSlug: string,
  projectSlug: string,
  enabled: boolean,
) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!enabled || workspaceSlug.length === 0 || projectSlug.length === 0) {
      return;
    }

    const controller = new AbortController();

    async function poll() {
      let cursor: string | undefined;

      while (!controller.signal.aborted) {
        try {
          const result = await waitForTaskChanges(workspaceSlug, projectSlug, cursor, {
            signal: controller.signal,
          });
          cursor = result.cursor;

          if (result.changed) {
            await Promise.all([
              queryClient.invalidateQueries({
                queryKey: queryKeys.tasks(workspaceSlug, projectSlug),
              }),
              queryClient.invalidateQueries({
                queryKey: queryKeys.taskGroups(workspaceSlug, projectSlug),
              }),
            ]);
          }
        } catch (error) {
          if (isAbortError(error)) {
            return;
          }
          await delay(POLL_RETRY_DELAY_MS, controller.signal).catch(() => undefined);
        }
      }
    }

    void poll();

    return () => {
      controller.abort();
    };
  }, [enabled, projectSlug, queryClient, workspaceSlug]);
}

export function useTaskGroups(workspaceSlug: string, projectSlug: string) {
  return useQuery({
    queryKey: queryKeys.taskGroups(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listTaskGroups(workspaceSlug, projectSlug, undefined, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
  });
}

export function useTaskGroup(workspaceSlug: string, projectSlug: string, groupId: string) {
  return useQuery({
    queryKey: queryKeys.taskGroup(workspaceSlug, projectSlug, groupId),
    queryFn: ({ signal }) => getTaskGroup(workspaceSlug, projectSlug, groupId, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0 && groupId.length > 0,
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
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
  });
}

export function useTask(workspaceSlug: string, projectSlug: string, taskId: string) {
  return useQuery({
    queryKey: queryKeys.task(workspaceSlug, projectSlug, taskId),
    queryFn: ({ signal }) => getTask(workspaceSlug, projectSlug, taskId, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0 && taskId.length > 0,
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

export function useDeleteTask(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (taskId: string) => deleteTask(workspaceSlug, projectSlug, taskId),
    onMutate: async (taskId) => {
      const tasksKey = queryKeys.tasks(workspaceSlug, projectSlug);
      const taskKey = queryKeys.task(workspaceSlug, projectSlug, taskId);

      await queryClient.cancelQueries({ queryKey: tasksKey });
      await queryClient.cancelQueries({ queryKey: taskKey });

      const previousTasks = queryClient.getQueryData<ApiListData<TaskDetail>>(tasksKey);
      const previousTask = queryClient.getQueryData<TaskDetail>(taskKey);

      queryClient.setQueryData<ApiListData<TaskDetail>>(tasksKey, (current) =>
        current
          ? {
              ...current,
              items: current.items.filter((task) => task.id !== taskId),
            }
          : current,
      );
      queryClient.removeQueries({ queryKey: taskKey });

      return { previousTasks, previousTask };
    },
    onError: (_error, _taskId, context) => {
      if (context?.previousTasks) {
        queryClient.setQueryData(queryKeys.tasks(workspaceSlug, projectSlug), context.previousTasks);
      }
      if (context?.previousTask) {
        queryClient.setQueryData(
          queryKeys.task(workspaceSlug, projectSlug, context.previousTask.id),
          context.previousTask,
        );
      }
    },
    onSuccess: () => {
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
    onMutate: async (payload) => {
      const tasksKey = queryKeys.tasks(workspaceSlug, projectSlug);
      const taskKey = queryKeys.task(workspaceSlug, projectSlug, taskId);

      await queryClient.cancelQueries({ queryKey: tasksKey });
      await queryClient.cancelQueries({ queryKey: taskKey });

      const previousTasks = queryClient.getQueryData<ApiListData<TaskDetail>>(tasksKey);
      const previousTask = queryClient.getQueryData<TaskDetail>(taskKey);

      queryClient.setQueryData<ApiListData<TaskDetail>>(tasksKey, (current) => {
        if (!current) {
          return current;
        }

        return {
          ...current,
          items: current.items.map((task) =>
            task.id === taskId
              ? { ...task, status: payload.status, updated_at: new Date().toISOString() }
              : task,
          ),
        };
      });

      queryClient.setQueryData<TaskDetail>(taskKey, (current) =>
        current ? { ...current, status: payload.status, updated_at: new Date().toISOString() } : current,
      );

      return { previousTasks, previousTask };
    },
    onError: (_error, _payload, context) => {
      if (context?.previousTasks) {
        queryClient.setQueryData(queryKeys.tasks(workspaceSlug, projectSlug), context.previousTasks);
      }
      if (context?.previousTask) {
        queryClient.setQueryData(queryKeys.task(workspaceSlug, projectSlug, taskId), context.previousTask);
      }
    },
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

export function useMoveTaskStatus(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ taskId, status }: { taskId: string; status: UpdateTaskStatusPayload['status'] }) =>
      updateTaskStatus(workspaceSlug, projectSlug, taskId, { status }),
    onMutate: async ({ taskId, status }) => {
      const tasksKey = queryKeys.tasks(workspaceSlug, projectSlug);
      const taskKey = queryKeys.task(workspaceSlug, projectSlug, taskId);

      await queryClient.cancelQueries({ queryKey: tasksKey });
      await queryClient.cancelQueries({ queryKey: taskKey });

      const previousTasks = queryClient.getQueryData<ApiListData<TaskDetail>>(tasksKey);
      const previousTask = queryClient.getQueryData<TaskDetail>(taskKey);

      queryClient.setQueryData<ApiListData<TaskDetail>>(tasksKey, (current) => {
        if (!current) {
          return current;
        }

        return {
          ...current,
          items: current.items.map((task) =>
            task.id === taskId
              ? { ...task, status, updated_at: new Date().toISOString() }
              : task,
          ),
        };
      });

      queryClient.setQueryData<TaskDetail>(taskKey, (current) =>
        current ? { ...current, status, updated_at: new Date().toISOString() } : current,
      );

      return { previousTasks, previousTask, taskId };
    },
    onError: (_error, _payload, context) => {
      if (context?.previousTasks) {
        queryClient.setQueryData(queryKeys.tasks(workspaceSlug, projectSlug), context.previousTasks);
      }
      if (context?.previousTask) {
        queryClient.setQueryData(
          queryKeys.task(workspaceSlug, projectSlug, context.taskId),
          context.previousTask,
        );
      }
    },
    onSuccess: (updated, variables) => {
      queryClient.setQueryData(
        queryKeys.task(workspaceSlug, projectSlug, variables.taskId),
        updated,
      );
      void queryClient.invalidateQueries({
        queryKey: queryKeys.tasks(workspaceSlug, projectSlug),
      });
    },
  });
}
