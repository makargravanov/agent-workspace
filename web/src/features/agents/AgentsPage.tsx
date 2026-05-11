import { KeyRound, Plus, Search, Trash2 } from 'lucide-react';
import type { FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useCreateAgent, useDeleteAgent, useAgents } from '../../hooks/useAgents';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { slugify } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';
import { useAutoSlug } from '../../shared/ui/useAutoSlug';

export function AgentsPage() {
  const { workspaceSlug = '' } = useParams();
  const navigate = useNavigate();
  const sessionQuery = useSession();
  const agentsQuery = useAgents(workspaceSlug);
  const createAgentMutation = useCreateAgent(workspaceSlug);
  const deleteAgentMutation = useDeleteAgent(workspaceSlug);
  const actorRole = sessionQuery.data?.actor?.role;
  const canEdit = actorRole === 'owner';
  const { value: displayName, setValue: setDisplayName, slug: key, setSlug: setKey } = useAutoSlug();
  const [search, setSearch] = useState('');

  const agents = agentsQuery.data?.items ?? [];
  const filteredAgents = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return agents;
    return agents.filter(
      (agent) =>
        agent.display_name.toLowerCase().includes(query) || agent.key.toLowerCase().includes(query),
    );
  }, [agents, search]);

  if (agentsQuery.isLoading) {
    return <FullPageMessage title="Загрузка агентов" embedded />;
  }

  if (agentsQuery.error) {
    return (
      <FullPageMessage
        title="Не удалось загрузить агентов"
        description={getErrorMessage(agentsQuery.error)}
        embedded
      />
    );
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createAgentMutation.mutate(
      { display_name: displayName.trim(), key: key.trim() },
      {
        onSuccess: (agent) => {
          setDisplayName('');
          setKey('');
          navigate(`/workspaces/${workspaceSlug}/agents/${agent.id}`);
        },
      },
    );
  }

  return (
    <section className="directoryPage">
      <div className="directoryHeader">
        <div className="searchField">
          <Search size={16} />
          <input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Поиск агентов" />
        </div>
      </div>

      <section className="directoryPanel">
        {filteredAgents.length > 0 ? (
          <div className="directoryList">
            {filteredAgents.map((agent) => (
              <div key={agent.id} className="directoryRow directoryRowWithActions">
                <Link className="directoryRowMain" to={`/workspaces/${workspaceSlug}/agents/${agent.id}`}>
                  <KeyRound size={18} />
                  <div>
                    <strong>{agent.display_name}</strong>
                    <span>{agent.key}</span>
                  </div>
                  <span className={`statusPill status-${agent.status}`}>{agent.status}</span>
                </Link>
                {canEdit ? (
                  <div className="rowActions">
                    <button
                      type="button"
                      className="iconButton dangerIconButton"
                      onClick={() => {
                        if (window.confirm(`Удалить агента «${agent.display_name}»?`)) {
                          deleteAgentMutation.mutate(agent.id);
                        }
                      }}
                      disabled={deleteAgentMutation.isPending}
                      aria-label={`Удалить агента ${agent.display_name}`}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                ) : null}
              </div>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">Агентов нет</div>
        )}
      </section>

      {deleteAgentMutation.error ? <p className="errorText">{getErrorMessage(deleteAgentMutation.error)}</p> : null}

      {canEdit ? (
        <section className="composePanel">
          <div className="compactTitle">
            <Plus size={16} />
            <h2>Создать агента</h2>
          </div>

          <form className="formGrid" onSubmit={handleSubmit}>
            <label className="field">
              <span>Отображаемое имя</span>
              <input
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
                placeholder="Automation Bot"
                required
              />
            </label>
            <label className="field">
              <span>Key</span>
              <input
                value={key}
                onChange={(event) => setKey(slugify(event.target.value))}
                placeholder="automation-bot"
                required
              />
            </label>
            <div className="formActions">
              <button type="submit" className="primaryButton compactButton" disabled={createAgentMutation.isPending}>
                {createAgentMutation.isPending ? 'Создание...' : 'Создать'}
              </button>
            </div>
          </form>

          {createAgentMutation.error ? <p className="errorText">{getErrorMessage(createAgentMutation.error)}</p> : null}
        </section>
      ) : null}
    </section>
  );
}
