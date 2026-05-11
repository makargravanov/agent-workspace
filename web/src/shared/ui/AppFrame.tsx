import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  BriefcaseBusiness,
  CheckSquare,
  FileText,
  FolderKanban,
  KeyRound,
  LayoutDashboard,
  LogOut,
  PanelLeftClose,
  PanelLeftOpen,
  StickyNote,
  Trash2,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { Link, NavLink, useNavigate, useParams } from 'react-router-dom';
import { logout } from '../../api/auth';
import { queryKeys } from '../../api/query-keys';
import {
  useDeleteProject,
  useDeleteWorkspace,
  useProject,
  useProjects,
  useWorkspace,
  useWorkspaces,
} from '../../hooks/useWorkspaces';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../lib/errors';

type ContextLink = {
  icon: LucideIcon;
  label: string;
  to: string;
  end?: boolean;
};

export function AppFrame({ children }: { children: ReactNode }) {
  const [railExpanded, setRailExpanded] = useState(false);
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
  const currentWorkspace = workspaces.find((workspace) => workspace.slug === workspaceSlug);
  const currentProject = projects.find((project) => project.slug === projectSlug);
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

  const contextLinks = getContextLinks(workspaceSlug, projectSlug);

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
    <div className={`appShell${railExpanded ? ' railExpanded' : ''}`}>
      <aside className={`appRail${railExpanded ? ' isExpanded' : ''}`}>
        <div className="appRailGroup">
          <div className="appRailHeader">
            <Link className="brandMark brandMarkRail" to="/workspaces" aria-label="Agent Workspace">
              <span className="brandIcon">AW</span>
              {railExpanded ? <span className="brandLabel">Agent Workspace</span> : null}
            </Link>
          </div>

          <nav className="railNav" aria-label="Основная навигация">
            <RailLink
              to="/workspaces"
              icon={BriefcaseBusiness}
              label="Рабочие пространства"
              end
              expanded={railExpanded}
            />
            {workspaceSlug ? (
              <RailLink
                to={`/workspaces/${workspaceSlug}`}
                icon={FolderKanban}
                label={currentWorkspace?.name ?? workspaceName}
                expanded={railExpanded}
              />
            ) : null}
            {workspaceSlug && projectSlug ? (
              <RailLink
                to={`/workspaces/${workspaceSlug}/projects/${projectSlug}`}
                icon={LayoutDashboard}
                label={currentProject?.name ?? projectName ?? projectSlug}
                expanded={railExpanded}
              />
            ) : null}
            {workspaceSlug ? (
              <RailLink
                to={`/workspaces/${workspaceSlug}/agents`}
                icon={KeyRound}
                label="Agents"
                expanded={railExpanded}
              />
            ) : null}
          </nav>
        </div>

        <div className="appRailFooter">
          <button
            type="button"
            className="railToggle railToggleFooter"
            onClick={() => setRailExpanded((value) => !value)}
            aria-label={railExpanded ? 'Свернуть боковую панель' : 'Раскрыть боковую панель'}
            title={railExpanded ? 'Свернуть' : 'Раскрыть'}
          >
            {railExpanded ? <PanelLeftClose size={16} /> : <PanelLeftOpen size={16} />}
          </button>
        </div>
      </aside>

      <main className="appMain">
        <header className="topBar">
          <div className="topBarMain">
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

            {contextLinks.length > 0 ? (
              <nav className="topTabs topTabsMinimal" aria-label="Навигация по разделу">
                {contextLinks.map((link) => (
                  <NavLink
                    key={link.to}
                    to={link.to}
                    end={link.end}
                    className={({ isActive }) => `topTab topTabMinimal${isActive ? ' isActive' : ''}`}
                  >
                    <link.icon size={16} />
                    <span>{link.label}</span>
                  </NavLink>
                ))}
              </nav>
            ) : null}
          </div>

          <div className="topBarActions">
            <div className="actorMeta">
              <span>{actor?.actor_kind ?? 'human'}</span>
              <strong>{actor?.role ?? 'member'}</strong>
            </div>
            {canDeleteWorkspace || canDeleteProject ? (
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

function RailLink({
  to,
  icon: Icon,
  label,
  end,
  expanded,
}: {
  to: string;
  icon: LucideIcon;
  label: string;
  end?: boolean;
  expanded: boolean;
}) {
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) => `railLink${isActive ? ' isActive' : ''}`}
      title={label}
      aria-label={label}
    >
      <Icon size={18} />
      {expanded ? <span className="railLinkLabel">{label}</span> : <span className="srOnly">{label}</span>}
    </NavLink>
  );
}

function getContextLinks(workspaceSlug: string, projectSlug: string): ContextLink[] {
  if (workspaceSlug && projectSlug) {
    return [
      {
        icon: LayoutDashboard,
        label: 'Обзор',
        to: `/workspaces/${workspaceSlug}/projects/${projectSlug}`,
        end: true,
      },
      {
        icon: CheckSquare,
        label: 'Задачи',
        to: `/workspaces/${workspaceSlug}/projects/${projectSlug}/tasks`,
      },
      {
        icon: StickyNote,
        label: 'Заметки',
        to: `/workspaces/${workspaceSlug}/projects/${projectSlug}/notes`,
      },
      {
        icon: FileText,
        label: 'Документы',
        to: `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`,
      },
    ];
  }

  if (workspaceSlug) {
    return [
      {
        icon: LayoutDashboard,
        label: 'Проекты',
        to: `/workspaces/${workspaceSlug}`,
        end: true,
      },
      {
        icon: KeyRound,
        label: 'Agents',
        to: `/workspaces/${workspaceSlug}/agents`,
      },
    ];
  }

  return [];
}
