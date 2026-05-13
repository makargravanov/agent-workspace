import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { listProjectActivity, listWorkspaceActivity } from '../api/activity';
import { queryKeys } from '../api/query-keys';

export function useWorkspaceActivity(workspaceSlug: string, page: number, perPage: number) {
  return useQuery({
    queryKey: queryKeys.workspaceActivity(workspaceSlug, page, perPage),
    queryFn: ({ signal }) =>
      listWorkspaceActivity(workspaceSlug, { page, per_page: perPage }, { signal }),
    enabled: workspaceSlug.length > 0,
    placeholderData: keepPreviousData,
  });
}

export function useProjectActivity(
  workspaceSlug: string,
  projectSlug: string,
  page: number,
  perPage: number,
) {
  return useQuery({
    queryKey: queryKeys.projectActivity(workspaceSlug, projectSlug, page, perPage),
    queryFn: ({ signal }) =>
      listProjectActivity(workspaceSlug, projectSlug, { page, per_page: perPage }, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
    placeholderData: keepPreviousData,
  });
}
