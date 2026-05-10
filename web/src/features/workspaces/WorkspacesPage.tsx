import { Plus, Search } from 'lucide-react';
import type { FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useCreateWorkspace, useWorkspaces } from '../../hooks/useWorkspaces';
import { getErrorMessage } from '../../shared/lib/errors';
import { slugify } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';
import { useAutoSlug } from '../../shared/ui/useAutoSlug';

export function WorkspacesPage() {
  const navigate = useNavigate();
  const workspacesQuery = useWorkspaces();
  const createWorkspaceMutation = useCreateWorkspace();
  const { value: name, setValue: setName, slug, setSlug } = useAutoSlug();
  const [search, setSearch] = useState('');

  if (workspacesQuery.isLoading) {
    return <FullPageMessage title="Загрузка рабочих пространств" embedded />;
  }

  if (workspacesQuery.error) {
    return (
      <FullPageMessage
        title="Не удалось загрузить рабочие пространства"
        description={getErrorMessage(workspacesQuery.error)}
        embedded
      />
    );
  }

  const workspaces = workspacesQuery.data?.items ?? [];
  const filteredWorkspaces = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) {
      return workspaces;
    }
    return workspaces.filter(
      (workspace) =>
        workspace.name.toLowerCase().includes(query) ||
        workspace.slug.toLowerCase().includes(query),
    );
  }, [search, workspaces]);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createWorkspaceMutation.mutate(
      { name: name.trim(), slug: slug.trim() },
      {
        onSuccess: (workspace) => {
          setName('');
          setSlug('');
          navigate(`/workspaces/${workspace.slug}`);
        },
      },
    );
  }

  return (
    <section className="directoryPage">
      <div className="directoryHeader">
        <div className="searchField">
          <Search size={16} />
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Поиск workspace"
          />
        </div>
      </div>

      <section className="directoryPanel">
        {filteredWorkspaces.length > 0 ? (
          <div className="directoryList">
            {filteredWorkspaces.map((workspace) => (
              <Link
                key={workspace.id}
                className="directoryRow"
                to={`/workspaces/${workspace.slug}`}
              >
                <div>
                  <strong>{workspace.name}</strong>
                  <span>{workspace.slug}</span>
                </div>
                <span>Открыть</span>
              </Link>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">Рабочих пространств нет</div>
        )}
      </section>

      <section className="composePanel">
        <div className="compactTitle">
          <Plus size={16} />
          <h2>Создать рабочее пространство</h2>
        </div>

        <form className="formGrid" onSubmit={handleSubmit}>
          <label className="field">
            <span>Название</span>
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Операции"
              required
            />
          </label>
          <label className="field">
            <span>Slug</span>
            <input
              value={slug}
              onChange={(event) => setSlug(slugify(event.target.value))}
              placeholder="operations"
              required
            />
          </label>
          <div className="formActions">
            <button
              type="submit"
              className="primaryButton compactButton"
              disabled={createWorkspaceMutation.isPending}
            >
              {createWorkspaceMutation.isPending ? 'Создание...' : 'Создать'}
            </button>
          </div>
        </form>

        {createWorkspaceMutation.error ? (
          <p className="errorText">{getErrorMessage(createWorkspaceMutation.error)}</p>
        ) : null}
      </section>
    </section>
  );
}
