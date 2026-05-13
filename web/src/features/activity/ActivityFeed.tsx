import { ChevronLeft, ChevronRight, History } from 'lucide-react';
import type { ActivityEvent } from '../../api/types';
import { formatDateTime } from '../../shared/lib/text';

export type ActivityDateWindow = 'all' | 'today' | '7d' | '30d';

export interface ActivityFiltersValue {
  entityType: string;
  actorType: string;
  dateWindow: ActivityDateWindow;
}

interface ActivityFeedProps {
  items: ActivityEvent[];
  filters: ActivityFiltersValue;
  onFiltersChange: (filters: ActivityFiltersValue) => void;
  isLoading: boolean;
  errorMessage?: string;
  page: number;
  hasNextPage: boolean;
  onPreviousPage: () => void;
  onNextPage: () => void;
}

const DATE_WINDOW_LABELS: Record<ActivityDateWindow, string> = {
  all: 'Все даты',
  today: 'Сегодня',
  '7d': '7 дней',
  '30d': '30 дней',
};

export function ActivityFeed({
  items,
  filters,
  onFiltersChange,
  isLoading,
  errorMessage,
  page,
  hasNextPage,
  onPreviousPage,
  onNextPage,
}: ActivityFeedProps) {
  const entityTypes = uniqueSorted(items.map((item) => item.entity_type));
  const actorTypes = uniqueSorted(items.map((item) => item.actor_type));
  const filteredItems = filterActivityItems(items, filters);

  return (
    <section className="activitySurface">
      <header className="activityHeader">
        <div className="compactTitle">
          <History size={16} />
          <h2>Активность</h2>
        </div>
        <ActivityFilters
          filters={filters}
          entityTypes={entityTypes}
          actorTypes={actorTypes}
          onFiltersChange={onFiltersChange}
        />
      </header>

      {errorMessage ? <div className="actionBanner errorBanner documentsInlineBanner">{errorMessage}</div> : null}

      <div className="activityList" aria-busy={isLoading}>
        {filteredItems.map((item) => (
          <article key={item.id} className="activityItem">
            <div className="activityItemIcon">{item.entity_type.slice(0, 1).toUpperCase()}</div>
            <div className="activityItemMain">
              <div className="activityItemTitle">
                <strong>{eventVerbLabel(item.event_type)}</strong>
                <span className="statusPill">{entityTypeLabel(item.entity_type)}</span>
                <span>{activityTitle(item)}</span>
              </div>
              <div className="activityItemMeta">
                {renderActor(item)}
                <span>{formatDateTime(item.occurred_at)}</span>
              </div>
            </div>
          </article>
        ))}

        {!isLoading && filteredItems.length === 0 ? (
          <div className="emptyPanel">Событий нет</div>
        ) : null}
        {isLoading ? <div className="emptyPanel">Загрузка...</div> : null}
      </div>

      <footer className="activityPagination">
        <button
          type="button"
          className="secondaryButton compactButton"
          onClick={onPreviousPage}
          disabled={page <= 1 || isLoading}
        >
          <ChevronLeft size={16} />
          Назад
        </button>
        <span className="mutedText">Страница {page}</span>
        <button
          type="button"
          className="secondaryButton compactButton"
          onClick={onNextPage}
          disabled={!hasNextPage || isLoading}
        >
          Вперед
          <ChevronRight size={16} />
        </button>
      </footer>
    </section>
  );
}

function ActivityFilters({
  filters,
  entityTypes,
  actorTypes,
  onFiltersChange,
}: {
  filters: ActivityFiltersValue;
  entityTypes: string[];
  actorTypes: string[];
  onFiltersChange: (filters: ActivityFiltersValue) => void;
}) {
  return (
    <div className="activityFilters">
      <label className="field compactField">
        <span>Сущность</span>
        <select
          value={filters.entityType}
          onChange={(event) => onFiltersChange({ ...filters, entityType: event.target.value })}
        >
          <option value="">Все</option>
          {entityTypes.map((entityType) => (
            <option key={entityType} value={entityType}>
              {entityTypeLabel(entityType)}
            </option>
          ))}
        </select>
      </label>
      <label className="field compactField">
        <span>Автор</span>
        <select
          value={filters.actorType}
          onChange={(event) => onFiltersChange({ ...filters, actorType: event.target.value })}
        >
          <option value="">Все</option>
          {actorTypes.map((actorType) => (
            <option key={actorType} value={actorType}>
              {actorType}
            </option>
          ))}
        </select>
      </label>
      <label className="field compactField">
        <span>Период</span>
        <select
          value={filters.dateWindow}
          onChange={(event) =>
            onFiltersChange({ ...filters, dateWindow: event.target.value as ActivityDateWindow })
          }
        >
          {Object.entries(DATE_WINDOW_LABELS).map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}

function filterActivityItems(items: ActivityEvent[], filters: ActivityFiltersValue) {
  return items.filter((item) => {
    const matchesEntity = !filters.entityType || item.entity_type === filters.entityType;
    const matchesActor = !filters.actorType || item.actor_type === filters.actorType;
    const matchesDate = isWithinDateWindow(item.occurred_at, filters.dateWindow);
    return matchesEntity && matchesActor && matchesDate;
  });
}

function isWithinDateWindow(value: string, dateWindow: ActivityDateWindow) {
  if (dateWindow === 'all') return true;

  const occurredAt = new Date(value).getTime();
  if (Number.isNaN(occurredAt)) return false;

  const now = Date.now();
  if (dateWindow === 'today') {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    return occurredAt >= start.getTime();
  }

  const days = dateWindow === '7d' ? 7 : 30;
  return occurredAt >= now - days * 24 * 60 * 60 * 1000;
}

function uniqueSorted(values: string[]) {
  return Array.from(new Set(values.filter(Boolean))).sort((a, b) => a.localeCompare(b));
}

function eventVerbLabel(eventType: string) {
  if (eventType.includes('create')) return 'Создано';
  if (eventType.includes('update') || eventType.includes('patch')) return 'Обновлено';
  if (eventType.includes('delete')) return 'Удалено';
  if (eventType.includes('revoke')) return 'Отозвано';
  return eventType;
}

function entityTypeLabel(entityType: string) {
  switch (entityType) {
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
      return entityType;
  }
}

function renderActor(item: ActivityEvent) {
  const githubLogin = githubLoginFromActivity(item);
  if (githubLogin) {
    return (
      <a
        className="assetUploaderLink"
        href={`https://github.com/${githubLogin}`}
        target="_blank"
        rel="noreferrer"
      >
        @{githubLogin}
      </a>
    );
  }

  return <span>{actorLabel(item)}</span>;
}

function actorLabel(item: ActivityEvent) {
  return `${item.actor_type}${item.actor_id ? ` ${item.actor_id.slice(0, 8)}` : ''}`;
}

function activityTitle(item: ActivityEvent) {
  const payload = parsePayload(item.payload_json);
  const title = payload.title ?? payload.name ?? payload.display_name ?? payload.file_name ?? payload.key;
  return typeof title === 'string' && title.trim().length > 0
    ? title
    : item.entity_id
      ? item.entity_id.slice(0, 8)
      : item.entity_type;
}

function parsePayload(payloadJson: string | null): Record<string, unknown> {
  if (!payloadJson) return {};
  try {
    const parsed = JSON.parse(payloadJson) as unknown;
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
}

function githubLoginFromActivity(item: ActivityEvent) {
  const payload = parsePayload(item.payload_json);
  for (const key of ['github_login', 'uploaded_by_github_login', 'actor_login', 'login']) {
    const value = payload[key];
    if (typeof value === 'string') {
      const login = normalizeGithubLogin(value);
      if (login) return login;
    }
  }

  return normalizeGithubLogin(item.actor_id);
}

function normalizeGithubLogin(value: string | null | undefined): string | null {
  const login = value?.trim() ?? '';
  if (!/^[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?$/.test(login)) {
    return null;
  }
  return login;
}
