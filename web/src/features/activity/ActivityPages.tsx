import { useMemo, useState } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import { useProjectActivity, useWorkspaceActivity } from '../../hooks/useActivity';
import { getErrorMessage } from '../../shared/lib/errors';
import { ActivityFeed, type ActivityFiltersValue } from './ActivityFeed';

const PER_PAGE = 20;

const DEFAULT_FILTERS: ActivityFiltersValue = {
  entityType: '',
  actorType: '',
  dateWindow: 'all',
};

export function WorkspaceActivityPage() {
  const { workspaceSlug = '' } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const [filters, setFilters] = useState(DEFAULT_FILTERS);
  const page = normalizePage(searchParams.get('page'));
  const activityQuery = useWorkspaceActivity(workspaceSlug, page, PER_PAGE);

  return (
    <ActivityFeed
      items={activityQuery.data?.items ?? []}
      filters={filters}
      onFiltersChange={setFilters}
      isLoading={activityQuery.isFetching}
      errorMessage={activityQuery.error ? getErrorMessage(activityQuery.error) : undefined}
      page={page}
      hasNextPage={Boolean(activityQuery.data?.next_cursor)}
      onPreviousPage={() => setPage(searchParams, setSearchParams, page - 1)}
      onNextPage={() => setPage(searchParams, setSearchParams, page + 1)}
    />
  );
}

export function ProjectActivityPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const [filters, setFilters] = useState(DEFAULT_FILTERS);
  const page = useMemo(() => normalizePage(searchParams.get('page')), [searchParams]);
  const activityQuery = useProjectActivity(workspaceSlug, projectSlug, page, PER_PAGE);

  return (
    <ActivityFeed
      items={activityQuery.data?.items ?? []}
      filters={filters}
      onFiltersChange={setFilters}
      isLoading={activityQuery.isFetching}
      errorMessage={activityQuery.error ? getErrorMessage(activityQuery.error) : undefined}
      page={page}
      hasNextPage={Boolean(activityQuery.data?.next_cursor)}
      onPreviousPage={() => setPage(searchParams, setSearchParams, page - 1)}
      onNextPage={() => setPage(searchParams, setSearchParams, page + 1)}
    />
  );
}

function normalizePage(value: string | null) {
  const page = Number(value ?? '1');
  return Number.isInteger(page) && page > 0 ? page : 1;
}

function setPage(
  searchParams: URLSearchParams,
  setSearchParams: ReturnType<typeof useSearchParams>[1],
  page: number,
) {
  const next = new URLSearchParams(searchParams);
  if (page <= 1) {
    next.delete('page');
  } else {
    next.set('page', String(page));
  }
  setSearchParams(next, { replace: true });
}
