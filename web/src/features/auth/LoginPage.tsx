import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Navigate, useLocation, useNavigate } from 'react-router-dom';
import { devLogin, getGithubStartUrl } from '../../api/auth';
import { queryKeys } from '../../api/query-keys';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';
import { enableDevLogin } from './config';

type LoginLocationState = {
  from?: string;
};

export function LoginPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const sessionQuery = useSession();
  const redirectTo = (location.state as LoginLocationState | null)?.from ?? '/workspaces';

  const loginMutation = useMutation({
    mutationFn: () => devLogin(),
    onSuccess: (session) => {
      queryClient.setQueryData(queryKeys.session(), session);
      void queryClient.invalidateQueries({ queryKey: queryKeys.workspaces() });
      navigate(redirectTo, { replace: true });
    },
  });

  if (sessionQuery.isLoading) {
    return <FullPageMessage title="Загрузка сессии" />;
  }

  if (sessionQuery.data?.authenticated) {
    return <Navigate to="/workspaces" replace />;
  }

  return (
    <main className="authPage">
      <section className="authCard">
        <div className="authCardHeader">
          <p className="eyebrow">agent-workspace</p>
          <h1>Вход в рабочее пространство</h1>
          <p className="mutedText">
            Основной вход выполняется через GitHub OAuth. Самостоятельной регистрации в интерфейсе нет.
          </p>
        </div>

        <div className="authButtons">
          <a className="primaryButton" href={getGithubStartUrl()}>
            Войти через GitHub
          </a>
          {enableDevLogin ? (
            <button
              type="button"
              className="secondaryButton"
              onClick={() => loginMutation.mutate()}
              disabled={loginMutation.isPending}
            >
              {loginMutation.isPending ? 'Выполняется вход...' : 'Dev login'}
            </button>
          ) : null}
        </div>

        {loginMutation.error ? (
          <p className="errorText">{getErrorMessage(loginMutation.error)}</p>
        ) : null}
      </section>
    </main>
  );
}
