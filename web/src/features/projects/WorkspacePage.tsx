import { FolderKanban, Plus, Search, Trash2 } from 'lucide-react';
import type { FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useCreateProject, useDeleteProject, useProjects, useWorkspace } from '../../hooks/useWorkspaces';
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
  const deleteProjectMutation = useDeleteProject(workspaceSlug);
  const actorRole = sessionQuery.data?.actor?.role;
  const canCreateProject = actorRole === 'owner';
  const canDeleteProject = actorRole === 'owner';
  const { value: name, setValue: setName, slug, setSlug } = useAutoSlug();
  const [search, setSearch] = useState('');
  const projects = projectsQuery.data?.items ?? [];
  const filteredProjects = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) {
      return projects;
    }
    return projects.filter(
      (project) =>
        project.name.toLowerCase().includes(query) ||
        project.slug.toLowerCase().includes(query),
    );
  }, [projects, search]);

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
    <section className="directoryPage">
      <div className="directoryHeader">
        <div className="searchField">
          <Search size={16} />
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Поиск проектов"
          />
        </div>
      </div>

      <section className="directoryPanel">
        {filteredProjects.length > 0 ? (
          <div className="directoryList">
            {filteredProjects.map((project) => (
              <div key={project.id} className="directoryRow directoryRowWithActions">
                <Link
                  className="directoryRowMain"
                  to={`/workspaces/${workspaceSlug}/projects/${project.slug}/tasks`}
                >
                  <FolderKanban size={18} />
                  <div>
                    <strong>{project.name}</strong>
                    <span>{project.slug}</span>
                  </div>
                  <span className={`statusPill status-${project.status}`}>
                    {projectStatusLabel(project.status)}
                  </span>
                </Link>
                {canDeleteProject ? (
                  <div className="rowActions">
                    <button
                      type="button"
                      className="iconButton dangerIconButton"
                      onClick={() => {
                        if (
                          window.confirm(
                            `Удалить проект «${project.name}»? Это действие необратимо.`,
                          )
                        ) {
                          deleteProjectMutation.mutate(project.slug);
                        }
                      }}
                      disabled={deleteProjectMutation.isPending}
                      title="Удалить проект"
                      aria-label={`Удалить проект ${project.name}`}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                ) : null}
              </div>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">Проектов нет</div>
        )}
      </section>

      {deleteProjectMutation.error ? (
        <p className="errorText">{getErrorMessage(deleteProjectMutation.error)}</p>
      ) : null}

      {canCreateProject ? (
        <section className="composePanel">
          <div className="compactTitle">
            <Plus size={16} />
            <h2>Создать проект</h2>
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
                className="primaryButton compactButton"
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
