import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import {
  createIntegrationConnection,
  deleteIntegrationConnection,
  listIntegrationConnections,
  updateIntegrationConnection,
} from '../api/integration-connections';
import type {
  CreateIntegrationConnectionPayload,
  IntegrationConnectionSummary,
  UpdateIntegrationConnectionPayload,
} from '../api/types';

export function useIntegrationConnections(workspaceSlug: string) {
  return useQuery({
    queryKey: queryKeys.integrationConnections(workspaceSlug),
    queryFn: ({ signal }) => listIntegrationConnections(workspaceSlug, undefined, { signal }),
    enabled: workspaceSlug.length > 0,
  });
}

export function useCreateIntegrationConnection(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateIntegrationConnectionPayload) =>
      createIntegrationConnection(workspaceSlug, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(
        queryKeys.integrationConnection(workspaceSlug, created.id),
        created,
      );
      void queryClient.invalidateQueries({
        queryKey: queryKeys.integrationConnections(workspaceSlug),
      });
    },
  });
}

export function useUpdateIntegrationConnection(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      connectionId,
      payload,
    }: {
      connectionId: string;
      payload: UpdateIntegrationConnectionPayload;
    }) => updateIntegrationConnection(workspaceSlug, connectionId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(
        queryKeys.integrationConnection(workspaceSlug, updated.id),
        updated,
      );
      void queryClient.invalidateQueries({
        queryKey: queryKeys.integrationConnections(workspaceSlug),
      });
    },
  });
}

export function useDeleteIntegrationConnection(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (connectionId: string) => deleteIntegrationConnection(workspaceSlug, connectionId),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.integrationConnections(workspaceSlug),
      });
    },
  });
}

export type IntegrationConnectionListItem = IntegrationConnectionSummary;
