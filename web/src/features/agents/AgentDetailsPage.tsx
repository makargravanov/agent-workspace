import { KeyRound, Plus, Save, Trash2 } from 'lucide-react';
import type { FormEvent } from 'react';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  useAgent,
  useAgentCredentials,
  useCreateAgentCredential,
  useDeleteAgent,
  useDeleteAgentCredential,
  useUpdateAgent,
  useUpdateAgentCredential,
} from '../../hooks/useAgents';
import { useProjects } from '../../hooks/useWorkspaces';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

const DEFAULT_SCOPES = [
  'tasks:read',
  'tasks:write_status',
  'task_groups:read',
  'documents:read',
  'assets:read',
  'notes:read',
  'notes:write',
  'audit:read_recent',
];

function scopeLabel(scope: string) {
  return scope.replace(/:/g, ' · ');
}

export function AgentDetailsPage() {
  const { workspaceSlug = '', agentId = '' } = useParams();
  const navigate = useNavigate();
  const sessionQuery = useSession();
  const agentQuery = useAgent(workspaceSlug, agentId);
  const credentialsQuery = useAgentCredentials(workspaceSlug, agentId);
  const projectsQuery = useProjects(workspaceSlug);
  const updateAgentMutation = useUpdateAgent(workspaceSlug, agentId);
  const deleteAgentMutation = useDeleteAgent(workspaceSlug);
  const createCredentialMutation = useCreateAgentCredential(workspaceSlug, agentId);
  const deleteCredentialMutation = useDeleteAgentCredential(workspaceSlug, agentId);
  const actorRole = sessionQuery.data?.actor?.role;
  const canEdit = actorRole === 'owner';
  const [showCreate, setShowCreate] = useState(false);
  const [secret, setSecret] = useState<string | null>(null);
  const [editingCredentialId, setEditingCredentialId] = useState<string | null>(null);
  const [label, setLabel] = useState('');
  const [projectId, setProjectId] = useState('');
  const [expiresAt, setExpiresAt] = useState('');
  const [scopes, setScopes] = useState<string[]>(['tasks:read']);
  const [editLabel, setEditLabel] = useState('');
  const [editProjectId, setEditProjectId] = useState('');
  const [editExpiresAt, setEditExpiresAt] = useState('');
  const [editScopes, setEditScopes] = useState<string[]>(['tasks:read']);
  const updateCredentialMutation = useUpdateAgentCredential(workspaceSlug, editingCredentialId ?? '');

  useEffect(() => {
    if (createCredentialMutation.data) {
      setSecret(createCredentialMutation.data.secret);
      setShowCreate(false);
    }
  }, [createCredentialMutation.data]);

  const projects = projectsQuery.data?.items ?? [];
  const agent = agentQuery.data;
  const credentials = credentialsQuery.data?.items ?? [];
  const credentialCount = credentials.length;
  const projectNameById = useMemo(() => new Map(projects.map((project) => [project.id, project.name])), [projects]);

  if (agentQuery.isLoading || credentialsQuery.isLoading) {
    return <FullPageMessage title="Загрузка агента" embedded />;
  }

  if (agentQuery.error || !agent) {
    return (
      <FullPageMessage
        title="Агент не найден"
        description={agentQuery.error ? getErrorMessage(agentQuery.error) : undefined}
        embedded
      />
    );
  }

  const currentAgent = agent;

  function toggleScope(scope: string) {
    setScopes((current) =>
      current.includes(scope) ? current.filter((item) => item !== scope) : [...current, scope],
    );
  }

  function resetCredentialForm() {
    setLabel('');
    setProjectId('');
    setExpiresAt('');
    setScopes(['tasks:read']);
  }

  function beginEdit(credentialId: string) {
    const current = credentials.find((item) => item.id === credentialId);
    if (!current) return;
    setEditingCredentialId(credentialId);
    setEditLabel(current.label);
    setEditProjectId(current.project_id ?? '');
    setEditExpiresAt(current.expires_at ?? '');
    setEditScopes(current.scope_policy);
  }

  function toggleEditScope(scope: string) {
    setEditScopes((current) =>
      current.includes(scope) ? current.filter((item) => item !== scope) : [...current, scope],
    );
  }

  function handleCredentialUpdate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editingCredentialId) return;
    updateCredentialMutation.mutate(
      {
        label: editLabel.trim(),
        project_id: editProjectId || null,
        scopes: editScopes,
        expires_at: editExpiresAt || null,
      },
      {
        onSuccess: () => {
          setEditingCredentialId(null);
        },
      },
    );
  }

  function handleAgentSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    updateAgentMutation.mutate({
      display_name: currentAgent.display_name,
      key: currentAgent.key,
      status: currentAgent.status === 'active' ? 'disabled' : 'active',
    });
  }

  function handleCredentialSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createCredentialMutation.mutate(
      {
        label: label.trim(),
        project_id: projectId || null,
        scopes,
        expires_at: expiresAt || null,
      },
      {
        onSuccess: (created) => {
          setSecret(created.secret);
          resetCredentialForm();
        },
      },
    );
  }

  return (
    <section className="overviewPage">
      <section className="composePanel">
        <div className="panelHeader">
          <div>
            <div className="compactTitle">
              <KeyRound size={16} />
              <h2>{currentAgent.display_name}</h2>
            </div>
            <p className="mutedText">{currentAgent.key}</p>
          </div>
          <span className={`statusPill status-${currentAgent.status}`}>{currentAgent.status}</span>
        </div>

        <div className="agentToolbar">
          {canEdit ? (
            <>
              <button
                type="button"
                className="secondaryButton compactButton"
                onClick={() => setShowCreate((value) => !value)}
              >
                <Plus size={16} />
                <span>Новый credential</span>
              </button>
              <button
                type="button"
                className="secondaryButton compactButton"
                onClick={() =>
                    updateAgentMutation.mutate({
                      key: currentAgent.key,
                      display_name: currentAgent.display_name,
                      status: currentAgent.status === 'active' ? 'disabled' : 'active',
                    })
                  }
                  disabled={updateAgentMutation.isPending}
                >
                  <Save size={16} />
                  <span>{currentAgent.status === 'active' ? 'Отключить' : 'Активировать'}</span>
                </button>
              <button
                type="button"
                className="iconButton dangerIconButton"
                onClick={() => {
                  if (window.confirm(`Удалить агента «${currentAgent.display_name}» вместе с credentials?`)) {
                    deleteAgentMutation.mutate(currentAgent.id, {
                      onSuccess: () => navigate(`/workspaces/${workspaceSlug}/agents`, { replace: true }),
                    });
                  }
                }}
                disabled={deleteAgentMutation.isPending}
                aria-label="Удалить агента"
              >
                <Trash2 size={16} />
              </button>
            </>
          ) : null}
        </div>

        <div className="agentMetaGrid">
          <div className="statCard">
            <span className="statLabel">Credentials</span>
            <strong className="statValue">{credentialCount}</strong>
          </div>
          <div className="statCard">
            <span className="statLabel">Создан</span>
            <strong>{new Date(currentAgent.created_at).toLocaleString()}</strong>
          </div>
          <div className="statCard">
            <span className="statLabel">Изменен</span>
            <strong>{new Date(currentAgent.updated_at).toLocaleString()}</strong>
          </div>
        </div>

        {canEdit ? (
          <form className="formGrid agentEditGrid" onSubmit={handleAgentSubmit}>
            <label className="field">
              <span>Display name</span>
              <input value={currentAgent.display_name} disabled />
            </label>
            <label className="field">
              <span>Key</span>
              <input value={currentAgent.key} disabled />
            </label>
          </form>
        ) : null}

        {updateAgentMutation.error ? <p className="errorText">{getErrorMessage(updateAgentMutation.error)}</p> : null}
        {deleteAgentMutation.error ? <p className="errorText">{getErrorMessage(deleteAgentMutation.error)}</p> : null}
      </section>

      {secret ? (
        <section className="composePanel secretPanel">
          <div className="compactTitle">
            <KeyRound size={16} />
            <h2>Secret создан</h2>
          </div>
          <p className="warningText">Этот secret будет показан только один раз.</p>
          <pre className="secretBox">{secret}</pre>
          <button type="button" className="primaryButton compactButton" onClick={() => void navigator.clipboard.writeText(secret)}>
            Копировать
          </button>
        </section>
      ) : null}

      {canEdit && showCreate ? (
        <section className="composePanel">
          <div className="compactTitle">
            <Plus size={16} />
            <h2>Создать credential</h2>
          </div>

          <form className="formGrid formGridWide" onSubmit={handleCredentialSubmit}>
            <label className="field">
              <span>Label</span>
              <input value={label} onChange={(event) => setLabel(event.target.value)} placeholder="cli-local" required />
            </label>
            <label className="field">
              <span>Project scope</span>
              <select value={projectId} onChange={(event) => setProjectId(event.target.value)}>
                <option value="">Workspace-wide</option>
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>Expires at</span>
              <input value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} placeholder="2026-12-31T23:59" />
            </label>
            <div className="field fieldSpan2">
              <span>Scopes</span>
              <div className="scopeGrid">
                {DEFAULT_SCOPES.map((scope) => (
                  <label key={scope} className="scopeChip">
                    <input type="checkbox" checked={scopes.includes(scope)} onChange={() => toggleScope(scope)} />
                    <span>{scopeLabel(scope)}</span>
                  </label>
                ))}
              </div>
            </div>
            <div className="formActions">
              <button type="submit" className="primaryButton compactButton" disabled={createCredentialMutation.isPending}>
                Создать secret
              </button>
            </div>
          </form>

          {createCredentialMutation.error ? <p className="errorText">{getErrorMessage(createCredentialMutation.error)}</p> : null}
        </section>
      ) : null}

      <section className="tablePanel">
        <table className="taskTable">
          <thead>
            <tr>
              <th>Label</th>
              <th>Scopes</th>
              <th>Status</th>
              <th>Project</th>
              <th>Expiry</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {credentials.map((credential) => (
              <tr key={credential.id}>
                <td>
                  <strong>{credential.label}</strong>
                  <span>{credential.secret_prefix}</span>
                </td>
                <td>{credential.scope_policy.map(scopeLabel).join(', ')}</td>
                <td><span className={`statusPill status-${credential.status}`}>{credential.status}</span></td>
                <td>{credential.project_id ? projectNameById.get(credential.project_id) ?? credential.project_id : 'Workspace-wide'}</td>
                <td>{credential.expires_at ? new Date(credential.expires_at).toLocaleString() : 'No expiry'}</td>
                <td>
                  {canEdit ? (
                    <div className="rowActions">
                      <button type="button" className="secondaryButton compactButton" onClick={() => beginEdit(credential.id)}>
                        Редактировать
                      </button>
                      <button
                        type="button"
                        className="iconButton dangerIconButton"
                        onClick={() => {
                          if (window.confirm(`Удалить credential «${credential.label}»?`)) {
                            deleteCredentialMutation.mutate(credential.id);
                          }
                        }}
                        disabled={deleteCredentialMutation.isPending}
                      >
                        <Trash2 size={16} />
                      </button>
                    </div>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {credentials.length === 0 ? <div className="emptyPanel">Credentials пока нет</div> : null}
      </section>

      {editingCredentialId ? (
        <section className="composePanel">
          <div className="compactTitle">
            <Save size={16} />
            <h2>Редактировать credential</h2>
          </div>
          <form className="formGrid formGridWide" onSubmit={handleCredentialUpdate}>
            <label className="field">
              <span>Label</span>
              <input value={editLabel} onChange={(event) => setEditLabel(event.target.value)} required />
            </label>
            <label className="field">
              <span>Project scope</span>
              <select value={editProjectId} onChange={(event) => setEditProjectId(event.target.value)}>
                <option value="">Workspace-wide</option>
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>Expires at</span>
              <input value={editExpiresAt} onChange={(event) => setEditExpiresAt(event.target.value)} />
            </label>
            <div className="field fieldSpan2">
              <span>Scopes</span>
              <div className="scopeGrid">
                {DEFAULT_SCOPES.map((scope) => (
                  <label key={scope} className="scopeChip">
                    <input type="checkbox" checked={editScopes.includes(scope)} onChange={() => toggleEditScope(scope)} />
                    <span>{scopeLabel(scope)}</span>
                  </label>
                ))}
              </div>
            </div>
            <div className="formActions">
              <button type="submit" className="primaryButton compactButton" disabled={updateCredentialMutation.isPending}>
                Сохранить
              </button>
              <button type="button" className="secondaryButton compactButton" onClick={() => setEditingCredentialId(null)}>
                Отмена
              </button>
            </div>
          </form>
          {updateCredentialMutation.error ? <p className="errorText">{getErrorMessage(updateCredentialMutation.error)}</p> : null}
        </section>
      ) : null}

      {credentialsQuery.error ? <p className="errorText">{getErrorMessage(credentialsQuery.error)}</p> : null}
    </section>
  );
}
