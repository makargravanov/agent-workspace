import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  BriefcaseBusiness,
  CheckSquare,
  FileText,
  FolderKanban,
  KeyRound,
  LayoutDashboard,
  LogOut,
  StickyNote,
  Trash2,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { Link, NavLink, useNavigate, useParams } from 'react-router-dom';
import { logout } from '../../api/auth';
import { queryKeys } from '../../api/query-keys';
import { useDeleteProject, useDeleteWorkspace, useProject, useProjects, useWorkspace, useWorkspaces } from '../../hooks/useWorkspaces';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../lib/errors';

export function AppFrame({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const sessionQuery = useSession();
  const workspacesQuery = useWorkspaces();
  const workspaceQuery = useWorkspace(workspaceSlug);
  const projectsQuery = useProjects(workspaceSlug);
  const projectQuery = useProject(workspaceSlug, projectSlug);
  const actor = sessionQuery.data?.actor;
  const workspaces = workspacesQuery.data?.items ?? [];
  const projects = projectsQuery.data?.items ?? [];
  const workspaceName = workspaceQuery.data?.name ?? 'Рабочие пространства';
  const projectName = projectQuery.data?.name;
  const deleteWorkspaceMutation = useDeleteWorkspace();
  const deleteProjectMutation = useDeleteProject(workspaceSlug);
  const canDeleteWorkspace = Boolean(workspaceSlug) && !projectSlug && actor?.role === 'owner';
  const canDeleteProject = Boolean(workspaceSlug && projectSlug) && actor?.role === 'owner';
  const deleting = deleteWorkspaceMutation.isPending || deleteProjectMutation.isPending;
  const deletionError = deleteProjectMutation.error ?? deleteWorkspaceMutation.error;

  const logoutMutation = useMutation({
    mutationFn: () => logout(),
    onSuccess: () => {
      queryClient.clear();
      queryClient.setQueryData(queryKeys.session(), { authenticated: false });
      navigate('/login', { replace: true });
    },
  });

  function handleDeleteCurrent() {
    if (projectSlug) {
      const label = projectName ?? projectSlug;
      if (!window.confirm(`Удалить проект «${label}»? Это действие необратимо.`)) {
        return;
      }

      deleteProjectMutation.mutate(projectSlug, {
        onSuccess: () => {
          navigate(`/workspaces/${workspaceSlug}`, { replace: true });
        },
      });
      return;
    }

    if (workspaceSlug) {
      const label = workspaceName ?? workspaceSlug;
      if (!window.confirm(`Удалить рабочее пространство «${label}»? Это действие необратимо.`)) {
        return;
      }

      deleteWorkspaceMutation.mutate(workspaceSlug, {
        onSuccess: () => {
          navigate('/workspaces', { replace: true });
        },
      });
    }
  }

  return (
    <div className="appShell">
      <aside className="appSidebar">
        <Link className="brandMark" to="/workspaces">
          <span className="brandIcon">AW</span>
          <span>Agent Workspace</span>
        </Link>

        <nav className="sidebarSection" aria-label="Рабочие пространства">
          <span className="sidebarLabel">Рабочие пространства</span>
          <NavLink to="/workspaces" className={({ isActive }) => `sidebarLink${isActive ? ' isActive' : ''}`}>
            <BriefcaseBusiness size={16} />
            <span>Все</span>
          </NavLink>
          {workspaces.slice(0, 6).map((workspace) => (
            <NavLink
              key={workspace.id}
              to={`/workspaces/${workspace.slug}`}
              className={({ isActive }) => `sidebarLink${isActive ? ' isActive' : ''}`}
            >
              <FolderKanban size={16} />
              <span>{workspace.name}</span>
            </NavLink>
          ))}
        </nav>

        {workspaceSlug ? (
          <nav className="sidebarSection" aria-label="Проекты">
            <span className="sidebarLabel">Проекты</span>
            {projects.length > 0 ? (
              projects.slice(0, 8).map((project) => (
                <NavLink
                  key={project.id}
                  to={`/workspaces/${workspaceSlug}/projects/${project.slug}/tasks`}
                  className={({ isActive }) => `sidebarLink${isActive ? ' isActive' : ''}`}
                >
                  <CheckSquare size={16} />
                  <span>{project.name}</span>
                </NavLink>
              ))
            ) : (
              <span className="sidebarEmpty">Проектов нет</span>
            )}
          </nav>
        ) : null}

        {workspaceSlug ? (
          <nav className="sidebarSection" aria-label="Агенты">
            <span className="sidebarLabel">Агенты</span>
            <NavLink
              to={`/workspaces/${workspaceSlug}/agents`}
              className={({ isActive }) => `sidebarLink${isActive ? ' isActive' : ''}`}
            >
              <KeyRound size={16} />
              <span>Agents</span>
            </NavLink>
          </nav>
        ) : null}
      </aside>

      <main className="appMain">
        <header className="topBar">
          <div className="topBarTitle">
            <div className="breadcrumbs">
              <Link to="/workspaces">Рабочие пространства</Link>
              {workspaceSlug ? <span>/</span> : null}
              {workspaceSlug ? <Link to={`/workspaces/${workspaceSlug}`}>{workspaceName}</Link> : null}
              {projectName ? <span>/</span> : null}
              {projectName ? <span>{projectName}</span> : null}
            </div>
            <h1>{projectName ?? workspaceName}</h1>
          </div>

          {workspaceSlug && projectSlug ? (
            <nav className="topTabs" aria-label="Навигация проекта">
              <NavLink
                to={`/workspaces/${workspaceSlug}/projects/${projectSlug}`}
                end
                className={({ isActive }) => `topTab${isActive ? ' isActive' : ''}`}
              >
                <LayoutDashboard size={16} />
                <span>Обзор</span>
              </NavLink>
              <NavLink
                to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/tasks`}
                className={({ isActive }) => `topTab${isActive ? ' isActive' : ''}`}
              >
                <CheckSquare size={16} />
                <span>Задачи</span>
              </NavLink>
              <NavLink
                to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/notes`}
                className={({ isActive }) => `topTab${isActive ? ' isActive' : ''}`}
              >
                <StickyNote size={16} />
                <span>Заметки</span>
              </NavLink>
              <NavLink
                to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`}
                className={({ isActive }) => `topTab${isActive ? ' isActive' : ''}`}
              >
                <FileText size={16} />
                <span>Документы</span>
              </NavLink>
            </nav>
          ) : null}

          <div className="topBarActions">
            <div className="actorMeta">
              <span>{actor?.actor_kind ?? 'human'}</span>
              <strong>{actor?.role ?? 'member'}</strong>
            </div>
            {(canDeleteWorkspace || canDeleteProject) ? (
              <button
                type="button"
                className="iconButton dangerIconButton"
                onClick={handleDeleteCurrent}
                disabled={deleting}
                title={canDeleteProject ? 'Удалить проект' : 'Удалить рабочее пространство'}
                aria-label={canDeleteProject ? 'Удалить проект' : 'Удалить рабочее пространство'}
              >
                <Trash2 size={18} />
              </button>
            ) : null}
            <button
              type="button"
              className="iconButton"
              onClick={() => logoutMutation.mutate()}
              disabled={logoutMutation.isPending}
              title="Выйти"
              aria-label="Выйти"
            >
              <LogOut size={18} />
            </button>
          </div>
        </header>

        {deletionError ? (
          <div className="actionBanner errorBanner">{getErrorMessage(deletionError)}</div>
        ) : null}

        <div className="appContent">{children}</div>
      </main>
    </div>
  );
}
