import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  createAsset,
  deleteAsset,
  getAsset,
  listAssets,
  updateAsset,
} from '../api/assets';
import { queryKeys } from '../api/query-keys';
import type {
  ApiListData,
  AssetDetail,
  CreateAssetPayload,
  PaginationParams,
  UpdateAssetPayload,
} from '../api/types';

export function useAssets(
  workspaceSlug: string,
  projectSlug: string,
  pagination?: PaginationParams,
) {
  return useQuery({
    queryKey: queryKeys.assets(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listAssets(workspaceSlug, projectSlug, pagination, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
  });
}

export function useAsset(workspaceSlug: string, projectSlug: string, assetId: string) {
  return useQuery({
    queryKey: queryKeys.asset(workspaceSlug, projectSlug, assetId),
    queryFn: ({ signal }) => getAsset(workspaceSlug, projectSlug, assetId, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0 && assetId.length > 0,
  });
}

export function useCreateAsset(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateAssetPayload) => createAsset(workspaceSlug, projectSlug, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(queryKeys.asset(workspaceSlug, projectSlug, created.id), created);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.assets(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useUpdateAsset(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ assetId, payload }: { assetId: string; payload: UpdateAssetPayload }) =>
      updateAsset(workspaceSlug, projectSlug, assetId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.asset(workspaceSlug, projectSlug, updated.id), updated);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.assets(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useDeleteAsset(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (assetId: string) => deleteAsset(workspaceSlug, projectSlug, assetId),
    onMutate: async (assetId) => {
      const assetsKey = queryKeys.assets(workspaceSlug, projectSlug);
      const assetKey = queryKeys.asset(workspaceSlug, projectSlug, assetId);

      await queryClient.cancelQueries({ queryKey: assetsKey });
      await queryClient.cancelQueries({ queryKey: assetKey });

      const previousAssets = queryClient.getQueryData<ApiListData<AssetDetail>>(assetsKey);
      const previousAsset = queryClient.getQueryData<AssetDetail>(assetKey);

      queryClient.setQueryData<ApiListData<AssetDetail>>(assetsKey, (current) =>
        current
          ? {
              ...current,
              items: current.items.filter((asset) => asset.id !== assetId),
            }
          : current,
      );
      queryClient.removeQueries({ queryKey: assetKey });

      return { previousAssets, previousAsset };
    },
    onError: (_error, _assetId, context) => {
      if (context?.previousAssets) {
        queryClient.setQueryData(
          queryKeys.assets(workspaceSlug, projectSlug),
          context.previousAssets,
        );
      }
      if (context?.previousAsset) {
        queryClient.setQueryData(
          queryKeys.asset(workspaceSlug, projectSlug, context.previousAsset.id),
          context.previousAsset,
        );
      }
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.assets(workspaceSlug, projectSlug),
      });
    },
  });
}
