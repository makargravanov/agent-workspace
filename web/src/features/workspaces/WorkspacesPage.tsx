import type { FormEvent } from 'react';
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
    <section className="pageStack">
      <div className="pageHeader">
        <div>
          <p className="eyebrow">Рабочие пространства</p>
          <h1>Доступные рабочие пространства</h1>
        </div>
      </div>

      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Список</h2>
          </div>
        </div>

        {workspaces.length > 0 ? (
          <div className="cardGrid">
            {workspaces.map((workspace) => (
              <Link
                key={workspace.id}
                className="summaryCard summaryCardLink"
                to={`/workspaces/${workspace.slug}`}
              >
                <strong>{workspace.name}</strong>
                <span className="mutedText">{workspace.slug}</span>
              </Link>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">
            <h3>Рабочих пространств пока нет</h3>
          </div>
        )}
      </section>

      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Создать рабочее пространство</h2>
          </div>
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
              className="primaryButton"
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
