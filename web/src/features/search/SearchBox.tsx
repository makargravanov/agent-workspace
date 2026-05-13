import { Search } from 'lucide-react';
import type { KeyboardEvent } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { ProjectSummary, SearchResult } from '../../api/types';
import { useSearch } from '../../hooks/useSearch';
import { getErrorMessage } from '../../shared/lib/errors';

interface SearchBoxProps {
  workspaceSlug: string;
  projectSlug?: string;
  projects: ProjectSummary[];
}

const KIND_ORDER = [
  'workspace',
  'project',
  'task',
  'task_group',
  'note',
  'document',
  'asset',
  'agent',
  'integration_connection',
];

export function SearchBox({ workspaceSlug, projectSlug, projects }: SearchBoxProps) {
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const debouncedQuery = useDebouncedValue(query, 250);
  const searchQuery = useSearch(workspaceSlug, projectSlug, debouncedQuery);
  const navigate = useNavigate();
  const results = useMemo(() => searchQuery.data?.items ?? [], [searchQuery.data?.items]);
  const groups = useMemo(() => groupResults(results), [results]);
  const safeActiveIndex = Math.min(activeIndex, Math.max(results.length - 1, 0));

  useEffect(() => {
    function handlePointerDown(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }

    document.addEventListener('pointerdown', handlePointerDown);
    return () => document.removeEventListener('pointerdown', handlePointerDown);
  }, []);

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (!open && ['ArrowDown', 'Enter'].includes(event.key)) {
      setOpen(true);
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setActiveIndex((value) => Math.min(value + 1, Math.max(results.length - 1, 0)));
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      setActiveIndex((value) => Math.max(value - 1, 0));
    }
    if (event.key === 'Escape') {
      setOpen(false);
    }
    if (event.key === 'Enter' && results[safeActiveIndex]) {
      event.preventDefault();
      openResult(results[safeActiveIndex]);
    }
  }

  function openResult(result: SearchResult) {
    const to = resultRoute(result, workspaceSlug, projectSlug, projects);
    setOpen(false);
    navigate(to);
  }

  const showPanel = open && workspaceSlug.length > 0 && query.trim().length > 0;

  return (
    <div className="globalSearch" ref={rootRef}>
      <div className="searchField globalSearchField">
        <Search size={16} />
        <input
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onKeyDown={handleKeyDown}
          placeholder={projectSlug ? 'Поиск в проекте' : 'Поиск в workspace'}
          aria-label="Поиск"
        />
      </div>

      {showPanel ? (
        <div className="searchResultsPanel">
          {query.trim().length === 1 ? (
            <div className="searchPanelState">Введите еще один символ</div>
          ) : null}
          {searchQuery.error ? (
            <div className="searchPanelState errorText">{getErrorMessage(searchQuery.error)}</div>
          ) : null}
          {searchQuery.isFetching && results.length === 0 ? (
            <div className="searchPanelState">Поиск...</div>
          ) : null}
          {!searchQuery.isFetching && debouncedQuery.trim().length > 1 && results.length === 0 && !searchQuery.error ? (
            <div className="searchPanelState">Ничего не найдено</div>
          ) : null}
          {groups.map((group) => (
            <section key={group.kind} className="searchResultGroup">
              <h3>{kindLabel(group.kind)}</h3>
              {group.items.map((item) => {
                const flatIndex = results.findIndex((result) => result.kind === item.kind && result.id === item.id);
                return (
                  <button
                    key={`${item.kind}-${item.id}`}
                    type="button"
                    className={`searchResultItem${flatIndex === safeActiveIndex ? ' isActive' : ''}`}
                    onMouseEnter={() => setActiveIndex(flatIndex)}
                    onClick={() => openResult(item)}
                  >
                    <span className="statusPill">{kindLabel(item.kind)}</span>
                    <span className="searchResultText">
                      <strong>{item.title}</strong>
                      <span>{item.summary?.trim() || projectContext(item, projects) || 'Без описания'}</span>
                    </span>
                  </button>
                );
              })}
            </section>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function useDebouncedValue(value: string, delayMs: number) {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    const timeoutId = window.setTimeout(() => setDebounced(value), delayMs);
    return () => window.clearTimeout(timeoutId);
  }, [delayMs, value]);

  return debounced;
}

function groupResults(results: SearchResult[]) {
  const groups = new Map<string, SearchResult[]>();
  for (const result of results) {
    groups.set(result.kind, [...(groups.get(result.kind) ?? []), result]);
  }

  return Array.from(groups.entries())
    .sort(([a], [b]) => KIND_ORDER.indexOf(a) - KIND_ORDER.indexOf(b))
    .map(([kind, items]) => ({ kind, items }));
}

function resultRoute(
  result: SearchResult,
  workspaceSlug: string,
  currentProjectSlug: string | undefined,
  projects: ProjectSummary[],
) {
  const projectSlug = result.project_id
    ? projects.find((project) => project.id === result.project_id)?.slug ?? currentProjectSlug
    : currentProjectSlug;

  switch (result.kind) {
    case 'workspace':
      return `/workspaces/${workspaceSlug}`;
    case 'project':
      return `/workspaces/${workspaceSlug}/projects/${projectSlug ?? currentProjectSlug ?? ''}`;
    case 'document':
      return projectSlug
        ? `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${result.id}`
        : `/workspaces/${workspaceSlug}`;
    case 'task':
      return projectSlug
        ? `/workspaces/${workspaceSlug}/projects/${projectSlug}/tasks?q=${encodeURIComponent(result.title)}`
        : `/workspaces/${workspaceSlug}`;
    case 'note':
      return projectSlug
        ? `/workspaces/${workspaceSlug}/projects/${projectSlug}/notes`
        : `/workspaces/${workspaceSlug}`;
    case 'asset':
      return projectSlug
        ? `/workspaces/${workspaceSlug}/projects/${projectSlug}/assets`
        : `/workspaces/${workspaceSlug}`;
    case 'agent':
      return `/workspaces/${workspaceSlug}/agents/${result.id}`;
    case 'integration_connection':
      return `/workspaces/${workspaceSlug}/integrations`;
    default:
      return projectSlug
        ? `/workspaces/${workspaceSlug}/projects/${projectSlug}`
        : `/workspaces/${workspaceSlug}`;
  }
}

function kindLabel(kind: string) {
  switch (kind) {
    case 'workspace':
      return 'Workspace';
    case 'project':
      return 'Project';
    case 'task':
      return 'Task';
    case 'task_group':
      return 'Group';
    case 'note':
      return 'Note';
    case 'document':
      return 'Doc';
    case 'asset':
      return 'File';
    case 'agent':
      return 'Agent';
    case 'integration_connection':
      return 'Integration';
    default:
      return kind;
  }
}

function projectContext(result: SearchResult, projects: ProjectSummary[]) {
  if (!result.project_id) return '';
  const project = projects.find((item) => item.id === result.project_id);
  return project ? project.name : '';
}
