import { NavLink, Outlet, useParams } from 'react-router-dom';
import { useProject, useWorkspace } from '../../hooks/useWorkspaces';
import { getErrorMessage } from '../../shared/lib/errors';
import { projectStatusLabel } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

export function ProjectRouteLayout() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const workspaceQuery = useWorkspace(workspaceSlug);
  const projectQuery = useProject(workspaceSlug, projectSlug);

  if (workspaceQuery.isLoading || projectQuery.isLoading) {
    return <FullPageMessage title="Загрузка проекта" embedded />;
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

  if (projectQuery.error || !projectQuery.data) {
    return (
      <FullPageMessage
        title="Проект не найден"
        description={projectQuery.error ? getErrorMessage(projectQuery.error) : undefined}
        embedded
      />
    );
  }

  return (
    <section className="pageStack">
      <div className="pageHeader">
        <div>
          <p className="eyebrow">Проект</p>
          <h1>{projectQuery.data.name}</h1>
          <p className="mutedText">
            {workspaceQuery.data.name} / {projectQuery.data.slug}
          </p>
        </div>
        <span className="statusBadge">{projectStatusLabel(projectQuery.data.status)}</span>
      </div>

      <nav className="tabNav" aria-label="Навигация по проекту">
        <NavLink
          to={`/workspaces/${workspaceSlug}/projects/${projectSlug}`}
          end
          className={({ isActive }) => `tabLink${isActive ? ' isActive' : ''}`}
        >
          Обзор
        </NavLink>
        <NavLink
          to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/tasks`}
          className={({ isActive }) => `tabLink${isActive ? ' isActive' : ''}`}
        >
          Задачи
        </NavLink>
        <NavLink
          to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/notes`}
          className={({ isActive }) => `tabLink${isActive ? ' isActive' : ''}`}
        >
          Заметки
        </NavLink>
      </nav>

      <Outlet />
    </section>
  );
}
