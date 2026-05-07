import type { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router-dom';
import { useSession } from '../../hooks/useSession';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

export function RootRedirect() {
  const sessionQuery = useSession();

  if (sessionQuery.isLoading) {
    return <FullPageMessage title="Загрузка сессии" />;
  }

  return (
    <Navigate
      to={sessionQuery.data?.authenticated ? '/workspaces' : '/login'}
      replace
    />
  );
}

export function RequireAuth({ children }: { children: ReactNode }) {
  const sessionQuery = useSession();
  const location = useLocation();

  if (sessionQuery.isLoading) {
    return <FullPageMessage title="Загрузка сессии" />;
  }

  if (!sessionQuery.data?.authenticated) {
    return <Navigate to="/login" replace state={{ from: location.pathname + location.search }} />;
  }

  return <>{children}</>;
}
