import type { FormEvent } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useCreateProject, useProjects, useWorkspace } from '../../hooks/useWorkspaces';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { projectStatusLabel, slugify } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';
import { useAutoSlug } from '../../shared/ui/useAutoSlug';

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
    return <FullPageMessage title="Загрузка рабочего пространства" embedded />;
  }

  if (workspaceQuery.error || !workspaceQuery.data) {
    return (
      <FullPageMessage
        title="Рабочее пространство не найдено"
        description={workspaceQuery.error ? getErrorMessage(workspaceQuery.error) : undefined}
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
          <p className="eyebrow">Рабочее пространство</p>
          <h1>{workspaceQuery.data.name}</h1>
          <p className="mutedText">{workspaceQuery.data.slug}</p>
        </div>
      </div>

      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Проекты</h2>
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
                  <span className="statusBadge">{projectStatusLabel(project.status)}</span>
                </div>
                <span className="mutedText">{project.slug}</span>
              </Link>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">
            <h3>Проектов пока нет</h3>
          </div>
        )}
      </section>

      {canCreateProject ? (
        <section className="panel">
          <div className="panelHeader">
            <div>
              <h2>Создать проект</h2>
            </div>
          </div>

          <form className="formGrid" onSubmit={handleSubmit}>
            <label className="field">
              <span>Название</span>
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
                {createProjectMutation.isPending ? 'Создание...' : 'Создать'}
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
