import { apiGet, type RequestOptions } from './client';
import type { ApiListResponse, SearchParams, SearchResult } from './types';

export async function searchWorkspace(
  params: SearchParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<SearchResult>['data']> {
  const resp = await apiGet<ApiListResponse<SearchResult>>(
    '/search',
    {
      q: params.q,
      workspace_slug: params.workspace_slug,
      project_slug: params.project_slug,
    },
    opts,
  );
  return resp.data;
}
