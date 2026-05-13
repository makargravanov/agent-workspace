import {
  CircleOff,
  Pencil,
  Plug,
  Plus,
  Save,
  Search,
  Trash2,
  X,
} from 'lucide-react';
import type { FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import type {
  IntegrationConnectionStatus,
  IntegrationConnectionSummary,
  IntegrationScopeKind,
  ProjectSummary,
} from '../../api/types';
import {
  useCreateIntegrationConnection,
  useDeleteIntegrationConnection,
  useIntegrationConnections,
  useUpdateIntegrationConnection,
} from '../../hooks/useIntegrationConnections';
import { useProjects } from '../../hooks/useWorkspaces';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

type ConnectionDraft = {
  provider: 'github';
  scope_kind: IntegrationScopeKind;
  project_id: string;
  status: IntegrationConnectionStatus;
  configText: string;
};

const EMPTY_CONFIG = '{\n  "repo": "owner/repo"\n}';

export function IntegrationConnectionsPage() {
  const { workspaceSlug = '' } = useParams();
  const sessionQuery = useSession();
  const connectionsQuery = useIntegrationConnections(workspaceSlug);
  const projectsQuery = useProjects(workspaceSlug);
  const createMutation = useCreateIntegrationConnection(workspaceSlug);
  const updateMutation = useUpdateIntegrationConnection(workspaceSlug);
  const deleteMutation = useDeleteIntegrationConnection(workspaceSlug);
  const [search, setSearch] = useState('');
  const [projectFilter, setProjectFilter] = useState('all');
  const [createOpen, setCreateOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const actorRole = sessionQuery.data?.actor?.role;
  const canMutate = actorRole === 'owner' || actorRole === 'editor';

  const connections = useMemo(
    () => connectionsQuery.data?.items ?? [],
    [connectionsQuery.data?.items],
  );
  const projects = useMemo(
    () => projectsQuery.data?.items ?? [],
    [projectsQuery.data?.items],
  );
  const filteredConnections = useMemo(() => {
    const query = search.trim().toLowerCase();
    return connections.filter((connection) => {
      const matchesProject =
        projectFilter === 'all' ||
        (projectFilter === 'workspace' && !connection.project_id) ||
        connection.project_id === projectFilter;
      const projectName = getProjectLabel(projects, connection.project_id).toLowerCase();
      const matchesSearch =
        !query ||
        connection.provider.toLowerCase().includes(query) ||
        connection.status.toLowerCase().includes(query) ||
        connection.scope_kind.toLowerCase().includes(query) ||
        projectName.includes(query);
      return matchesProject && matchesSearch;
    });
  }, [connections, projectFilter, projects, search]);

  if (connectionsQuery.isLoading || projectsQuery.isLoading) {
    return <FullPageMessage title="Loading integration connections" embedded />;
  }

  if (connectionsQuery.error) {
    return (
      <FullPageMessage
        title="Could not load integration connections"
        description={getErrorMessage(connectionsQuery.error)}
        embedded
      />
    );
  }

  function handleCreate(draft: ConnectionDraft) {
    const parsed = parseConfig(draft.configText);
    if (!parsed.ok) {
      return parsed.message;
    }

    createMutation.mutate(
      {
        provider: draft.provider,
        scope_kind: draft.scope_kind,
        project_id: draft.scope_kind === 'project' ? draft.project_id : null,
        status: draft.status,
        config_json: parsed.value,
      },
      {
        onSuccess: () => {
          setCreateOpen(false);
        },
      },
    );
    return null;
  }

  function handleUpdate(connectionId: string, draft: ConnectionDraft) {
    const parsed = parseConfig(draft.configText);
    if (!parsed.ok) {
      return parsed.message;
    }

    updateMutation.mutate(
      {
        connectionId,
        payload: {
          status: draft.status,
          config_json: parsed.value,
        },
      },
      {
        onSuccess: () => {
          setEditingId(null);
        },
      },
    );
    return null;
  }

  return (
    <section className="directoryPage integrationsPage">
      <div className="directoryHeader integrationsToolbar">
        <div className="searchField">
          <Search size={16} />
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search connections"
          />
        </div>
        <select value={projectFilter} onChange={(event) => setProjectFilter(event.target.value)}>
          <option value="all">All scopes</option>
          <option value="workspace">Workspace scope</option>
          {projects.map((project) => (
            <option key={project.id} value={project.id}>
              {project.name}
            </option>
          ))}
        </select>
        {canMutate ? (
          <button
            type="button"
            className="primaryButton compactButton"
            onClick={() => setCreateOpen((value) => !value)}
          >
            {createOpen ? <X size={16} /> : <Plus size={16} />}
            <span>{createOpen ? 'Close' : 'Add connection'}</span>
          </button>
        ) : null}
      </div>

      {createOpen && canMutate ? (
        <section className="composePanel">
          <div className="compactTitle">
            <Plug size={16} />
            <h2>New connection</h2>
          </div>
          <IntegrationConnectionForm
            projects={projects}
            mode="create"
            pending={createMutation.isPending}
            onCancel={() => setCreateOpen(false)}
            onSubmit={handleCreate}
          />
          {createMutation.error ? (
            <p className="errorText">{getErrorMessage(createMutation.error)}</p>
          ) : null}
        </section>
      ) : null}

      <section className="tablePanel">
        <IntegrationConnectionsTable
          connections={filteredConnections}
          projects={projects}
          canMutate={canMutate}
          editingId={editingId}
          mutationPending={updateMutation.isPending || deleteMutation.isPending}
          onEdit={setEditingId}
          onCancelEdit={() => setEditingId(null)}
          onUpdate={handleUpdate}
          onDelete={(connection) => {
            const label = `${connection.provider} / ${getProjectLabel(projects, connection.project_id)}`;
            if (window.confirm(`Delete integration connection "${label}"?`)) {
              deleteMutation.mutate(connection.id);
            }
          }}
        />
      </section>

      {updateMutation.error ? <p className="errorText">{getErrorMessage(updateMutation.error)}</p> : null}
      {deleteMutation.error ? <p className="errorText">{getErrorMessage(deleteMutation.error)}</p> : null}
    </section>
  );
}

export function IntegrationConnectionsTable({
  connections,
  projects,
  canMutate,
  editingId,
  mutationPending,
  onEdit,
  onCancelEdit,
  onUpdate,
  onDelete,
}: {
  connections: IntegrationConnectionSummary[];
  projects: ProjectSummary[];
  canMutate: boolean;
  editingId: string | null;
  mutationPending: boolean;
  onEdit: (connectionId: string | null) => void;
  onCancelEdit: () => void;
  onUpdate: (connectionId: string, draft: ConnectionDraft) => string | null;
  onDelete: (connection: IntegrationConnectionSummary) => void;
}) {
  if (connections.length === 0) {
    return <div className="emptyPanel integrationsEmpty">No integration connections</div>;
  }

  return (
    <table className="taskTable integrationsTable">
      <thead>
        <tr>
          <th>Provider</th>
          <th>Scope</th>
          <th>Project</th>
          <th>Status</th>
          <th>Config</th>
          <th>Updated</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        {connections.map((connection) =>
          editingId === connection.id ? (
            <tr key={connection.id}>
              <td colSpan={7}>
                <IntegrationConnectionForm
                  projects={projects}
                  mode="edit"
                  connection={connection}
                  pending={mutationPending}
                  onCancel={onCancelEdit}
                  onSubmit={(draft) => onUpdate(connection.id, draft)}
                />
              </td>
            </tr>
          ) : (
            <tr key={connection.id}>
              <td>
                <strong>{connection.provider}</strong>
                <span>{connection.id}</span>
              </td>
              <td>{scopeLabel(connection.scope_kind)}</td>
              <td>{getProjectLabel(projects, connection.project_id)}</td>
              <td>
                <span className={`statusPill status-${connection.status}`}>
                  {connection.status}
                </span>
              </td>
              <td className="integrationsConfigCell">{summarizeConfig(connection.config_json)}</td>
              <td>{formatDate(connection.updated_at)}</td>
              <td>
                {canMutate ? (
                  <div className="tableActionsCell">
                    <button
                      type="button"
                      className="iconButton"
                      onClick={() => onEdit(connection.id)}
                      title="Edit"
                      aria-label={`Edit ${connection.provider} connection`}
                    >
                      <Pencil size={16} />
                    </button>
                    <button
                      type="button"
                      className="iconButton dangerIconButton"
                      onClick={() => onDelete(connection)}
                      disabled={mutationPending}
                      title="Delete"
                      aria-label={`Delete ${connection.provider} connection`}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                ) : (
                  <span className="mutedText">Read only</span>
                )}
              </td>
            </tr>
          ),
        )}
      </tbody>
    </table>
  );
}

export function IntegrationConnectionForm({
  projects,
  mode,
  connection,
  pending,
  onCancel,
  onSubmit,
}: {
  projects: ProjectSummary[];
  mode: 'create' | 'edit';
  connection?: IntegrationConnectionSummary;
  pending: boolean;
  onCancel: () => void;
  onSubmit: (draft: ConnectionDraft) => string | null;
}) {
  const [draft, setDraft] = useState<ConnectionDraft>(() =>
    connection
      ? {
          provider: 'github',
          scope_kind: connection.scope_kind === 'project' ? 'project' : 'workspace',
          project_id: connection.project_id ?? '',
          status: normalizeStatus(connection.status),
          configText: formatConfig(connection.config_json),
        }
      : {
          provider: 'github',
          scope_kind: 'workspace',
          project_id: '',
          status: 'active',
          configText: EMPTY_CONFIG,
        },
  );
  const [validationError, setValidationError] = useState<string | null>(null);
  const isEdit = mode === 'edit';

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (draft.scope_kind === 'project' && !draft.project_id) {
      setValidationError('Project scope requires a project.');
      return;
    }
    const error = onSubmit(draft);
    setValidationError(error);
  }

  return (
    <form className="formGrid formGridWide integrationsForm" onSubmit={handleSubmit}>
      <label className="field">
        <span>Provider</span>
        <select
          value={draft.provider}
          onChange={(event) =>
            setDraft((current) => ({ ...current, provider: event.target.value as 'github' }))
          }
          disabled={isEdit}
        >
          <option value="github">GitHub</option>
        </select>
      </label>
      <label className="field">
        <span>Scope kind</span>
        <select
          value={draft.scope_kind}
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              scope_kind: event.target.value as IntegrationScopeKind,
              project_id: event.target.value === 'workspace' ? '' : current.project_id,
            }))
          }
          disabled={isEdit}
        >
          <option value="workspace">Workspace</option>
          <option value="project">Project</option>
        </select>
      </label>
      {draft.scope_kind === 'project' ? (
        <label className="field">
          <span>Project</span>
          <select
            value={draft.project_id}
            onChange={(event) =>
              setDraft((current) => ({ ...current, project_id: event.target.value }))
            }
            disabled={isEdit}
            required
          >
            <option value="">Select project</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      <label className="field">
        <span>Status</span>
        <select
          value={draft.status}
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              status: event.target.value as IntegrationConnectionStatus,
            }))
          }
        >
          <option value="active">Active</option>
          <option value="disabled">Disabled</option>
          <option value="error">Error</option>
        </select>
      </label>
      <label className="field fieldSpan2">
        <span>Config JSON</span>
        <textarea
          value={draft.configText}
          onChange={(event) =>
            setDraft((current) => ({ ...current, configText: event.target.value }))
          }
          rows={8}
          spellCheck={false}
          placeholder='{"repo":"owner/repo"}'
        />
      </label>
      <div className="formActions integrationsFormActions">
        <button type="submit" className="primaryButton compactButton" disabled={pending}>
          <Save size={16} />
          <span>{pending ? 'Saving...' : 'Save'}</span>
        </button>
        <button type="button" className="secondaryButton compactButton" onClick={onCancel}>
          <CircleOff size={16} />
          <span>Cancel</span>
        </button>
      </div>
      {validationError ? <p className="errorText fieldSpan2">{validationError}</p> : null}
    </form>
  );
}

function parseConfig(value: string): { ok: true; value: unknown } | { ok: false; message: string } {
  if (!value.trim()) {
    return { ok: true, value: {} };
  }

  try {
    return { ok: true, value: JSON.parse(value) as unknown };
  } catch {
    return { ok: false, message: 'Config JSON is invalid.' };
  }
}

function formatConfig(value: string | null) {
  if (!value) {
    return '';
  }

  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

function summarizeConfig(value: string | null) {
  if (!value) {
    return 'empty';
  }

  try {
    const parsed = JSON.parse(value) as Record<string, unknown>;
    const keys = Object.keys(parsed);
    return keys.length > 0 ? keys.join(', ') : 'empty object';
  } catch {
    return 'invalid JSON';
  }
}

function getProjectLabel(projects: ProjectSummary[], projectId: string | null) {
  if (!projectId) {
    return 'Workspace';
  }
  return projects.find((project) => project.id === projectId)?.name ?? projectId;
}

function normalizeStatus(status: string): IntegrationConnectionStatus {
  return status === 'disabled' || status === 'error' ? status : 'active';
}

function scopeLabel(scopeKind: string) {
  return scopeKind === 'project' ? 'Project' : 'Workspace';
}

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
}
