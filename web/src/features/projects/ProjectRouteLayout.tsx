import { Outlet, useParams } from 'react-router-dom';
import { useProject, useWorkspace } from '../../hooks/useWorkspaces';
import { getErrorMessage } from '../../shared/lib/errors';
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

  return <Outlet />;
}
