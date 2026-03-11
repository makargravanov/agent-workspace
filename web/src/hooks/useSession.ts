import { useQuery } from '@tanstack/react-query';
import { getSession } from '../api/auth';
import { queryKeys } from '../api/query-keys';

/**
 * Returns the current human session.
 * `data` is undefined while loading or when unauthenticated (401 triggers an ApiError).
 */
export function useSession() {
  return useQuery({
    queryKey: queryKeys.session(),
    queryFn: ({ signal }) => getSession({ signal }),
    // Session is stable; revalidate only on focus/reconnect.
    staleTime: 5 * 60 * 1000,
    // A 401 is expected when the user is not logged in — do not retry.
    retry: false,
  });
}
