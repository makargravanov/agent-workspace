import { useQuery } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import { searchWorkspace } from '../api/search';

export function useSearch(workspaceSlug: string, projectSlug: string | undefined, query: string) {
  const trimmedQuery = query.trim();

  return useQuery({
    queryKey: queryKeys.search(workspaceSlug, projectSlug, trimmedQuery),
    queryFn: ({ signal }) =>
      searchWorkspace(
        {
          q: trimmedQuery,
          workspace_slug: workspaceSlug,
          project_slug: projectSlug,
        },
        { signal },
      ),
    enabled: workspaceSlug.length > 0 && trimmedQuery.length > 1,
  });
}
