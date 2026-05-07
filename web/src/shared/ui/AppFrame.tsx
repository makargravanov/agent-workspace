import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { logout } from '../../api/auth';
import { queryKeys } from '../../api/query-keys';
import { useSession } from '../../hooks/useSession';

export function AppFrame({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const sessionQuery = useSession();
  const actor = sessionQuery.data?.actor;

  const logoutMutation = useMutation({
    mutationFn: () => logout(),
    onSuccess: () => {
      queryClient.clear();
      queryClient.setQueryData(queryKeys.session(), { authenticated: false });
      navigate('/login', { replace: true });
    },
  });

  return (
    <main className="appShell">
      <header className="appHeader">
        <div>
          <p className="eyebrow">agent-workspace</p>
          <div className="headerRow">
            <h1>Рабочее пространство</h1>
            <Link className="headerLink" to="/workspaces">
              Workspaces
            </Link>
          </div>
        </div>

        <div className="headerActions">
          <div className="actorMeta">
            <span>{actor?.actor_kind ?? 'human'}</span>
            <strong>{actor?.role ?? 'member'}</strong>
          </div>
          <button
            type="button"
            className="secondaryButton"
            onClick={() => logoutMutation.mutate()}
            disabled={logoutMutation.isPending}
          >
            {logoutMutation.isPending ? 'Выход...' : 'Выйти'}
          </button>
        </div>
      </header>

      <div className="appContent">{children}</div>
    </main>
  );
}
