import type { FormEvent } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useCreateProject, useProjects, useWorkspace } from '../../hooks/useWorkspaces';
import { slugify } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';
import { useAutoSlug } from '../../shared/ui/useAutoSlug';
import { getErrorMessage } from '../../shared/lib/errors';
import { useSession } from '../../hooks/useSession';

export function WorkspacePage() {
  const { workspaceSlug = '' } = useParams();
  const navigate = useNavigate();
  const sessionQuery = useSession();
  const workspaceQuery = useWorkspace(workspaceSlug);
  const projectsQuery = useProjects(workspaceSlug);
  const createProjectMutation = useCreateProject(workspaceSlug);
  const actorRole = sessionQuery.data?.actor?.role;
  const canCreateProject = actorRole === 'owner';
  const { value: name, setValue: setName, slug, setSlug } = useAutoSlug();

  if (workspaceQuery.isLoading || projectsQuery.isLoading) {
    return <FullPageMessage title="Загрузка workspace" embedded />;
  }

  if (workspaceQuery.error || !workspaceQuery.data) {
    return (
      <FullPageMessage
        title="Workspace не найден"
        description="Проверь slug workspace или доступ к нему."
        embedded
      />
    );
  }

  const projects = projectsQuery.data?.items ?? [];

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createProjectMutation.mutate(
      { name: name.trim(), slug: slug.trim() },
      {
        onSuccess: (project) => {
          setName('');
          setSlug('');
          navigate(`/workspaces/${workspaceSlug}/projects/${project.slug}`);
        },
      },
    );
  }

  return (
    <section className="pageStack">
      <div className="pageHeader">
        <div>
          <p className="eyebrow">Workspace</p>
          <h1>{workspaceQuery.data.name}</h1>
          <p className="mutedText">{workspaceQuery.data.slug}</p>
        </div>
      </div>

      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Проекты</h2>
            <p className="mutedText">Выбери проект или создай новый в этом workspace.</p>
          </div>
        </div>

        {projects.length > 0 ? (
          <div className="cardGrid">
            {projects.map((project) => (
              <Link
                key={project.id}
                className="summaryCard summaryCardLink"
                to={`/workspaces/${workspaceSlug}/projects/${project.slug}`}
              >
                <div className="summaryRow">
                  <strong>{project.name}</strong>
                  <span className="statusBadge">{project.status}</span>
                </div>
                <span className="mutedText">{project.slug}</span>
              </Link>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">
            <h3>В этом workspace пока нет проектов</h3>
            <p>Создай первый проект, чтобы перейти к задачам и заметкам.</p>
          </div>
        )}
      </section>

      {canCreateProject ? (
        <section className="panel">
          <div className="panelHeader">
            <div>
              <h2>Создать проект</h2>
              <p className="mutedText">Форма создания проекта вынесена на уровень workspace.</p>
            </div>
          </div>

          <form className="formGrid" onSubmit={handleSubmit}>
            <label className="field">
              <span>Название проекта</span>
              <input
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="Бэкенд платформы"
                required
              />
            </label>
            <label className="field">
              <span>Slug</span>
              <input
                value={slug}
                onChange={(event) => setSlug(slugify(event.target.value))}
                placeholder="backend-platform"
                required
              />
            </label>
            <div className="formActions">
              <button
                type="submit"
                className="primaryButton"
                disabled={createProjectMutation.isPending}
              >
                {createProjectMutation.isPending ? 'Создание...' : 'Создать проект'}
              </button>
            </div>
          </form>

          {createProjectMutation.error ? (
            <p className="errorText">{getErrorMessage(createProjectMutation.error)}</p>
          ) : null}
        </section>
      ) : null}
    </section>
  );
}
