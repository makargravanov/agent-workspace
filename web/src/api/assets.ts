import { apiDelete, apiGet, apiPatch, apiPost, apiUrl, type RequestOptions } from './client';
import type {
  ApiListResponse,
  ApiResponse,
  AssetDetail,
  CreateAssetPayload,
  PaginationParams,
  UpdateAssetPayload,
} from './types';

const assetsBase = (workspaceSlug: string, projectSlug: string) =>
  `/workspaces/${workspaceSlug}/projects/${projectSlug}/assets`;

export async function listAssets(
  workspaceSlug: string,
  projectSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<AssetDetail>['data']> {
  const resp = await apiGet<ApiListResponse<AssetDetail>>(
    assetsBase(workspaceSlug, projectSlug),
    pagination,
    opts,
  );
  return resp.data;
}

export async function getAsset(
  workspaceSlug: string,
  projectSlug: string,
  assetId: string,
  opts?: RequestOptions,
): Promise<AssetDetail> {
  const resp = await apiGet<ApiResponse<AssetDetail>>(
    `${assetsBase(workspaceSlug, projectSlug)}/${assetId}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createAsset(
  workspaceSlug: string,
  projectSlug: string,
  payload: CreateAssetPayload,
  opts?: RequestOptions,
): Promise<AssetDetail> {
  const resp = await apiPost<CreateAssetPayload, ApiResponse<AssetDetail>>(
    assetsBase(workspaceSlug, projectSlug),
    payload,
    opts,
  );
  return resp.data;
}

export async function updateAsset(
  workspaceSlug: string,
  projectSlug: string,
  assetId: string,
  payload: UpdateAssetPayload,
  opts?: RequestOptions,
): Promise<AssetDetail> {
  const resp = await apiPatch<UpdateAssetPayload, ApiResponse<AssetDetail>>(
    `${assetsBase(workspaceSlug, projectSlug)}/${assetId}`,
    payload,
    opts,
  );
  return resp.data;
}

export async function deleteAsset(
  workspaceSlug: string,
  projectSlug: string,
  assetId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(`${assetsBase(workspaceSlug, projectSlug)}/${assetId}`, opts);
}

export function assetDownloadUrl(
  workspaceSlug: string,
  projectSlug: string,
  assetId: string,
): string {
  return apiUrl(`${assetsBase(workspaceSlug, projectSlug)}/${assetId}/download`);
}
