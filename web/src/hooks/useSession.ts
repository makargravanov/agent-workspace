import { useQuery } from '@tanstack/react-query';
import { getSession } from '../api/auth';
import { queryKeys } from '../api/query-keys';

export function useSession() {
  return useQuery({
    queryKey: queryKeys.session(),
    queryFn: ({ signal }) => getSession({ signal }),
    staleTime: 5 * 60 * 1000,
    retry: false,
  });
}
