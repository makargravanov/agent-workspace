import { Plus, Trash2, Users } from 'lucide-react';
import type { FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import {
  useDeleteProjectMember,
  useProjectMembers,
  useUpsertProjectMember,
  useWorkspaceMembers,
} from '../../hooks/useMembers';
import { getErrorMessage } from '../../shared/lib/errors';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

export function ProjectMembersPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const workspaceMembersQuery = useWorkspaceMembers(workspaceSlug);
  const projectMembersQuery = useProjectMembers(workspaceSlug, projectSlug);
  const upsertMutation = useUpsertProjectMember(workspaceSlug, projectSlug);
  const deleteMutation = useDeleteProjectMember(workspaceSlug, projectSlug);
  const [memberId, setMemberId] = useState('');
  const [role, setRole] = useState<'editor' | 'viewer'>('editor');

  const workspaceMembers = useMemo(
    () => workspaceMembersQuery.data?.items.filter((member) => member.status === 'active' && member.role !== 'owner') ?? [],
    [workspaceMembersQuery.data?.items],
  );
  const projectMembers = useMemo(() => projectMembersQuery.data?.items ?? [], [projectMembersQuery.data?.items]);
  const projectMemberIds = useMemo(
    () => new Set(projectMembers.map((member) => member.workspace_member_id)),
    [projectMembers],
  );
  const candidates = workspaceMembers.filter((member) => !projectMemberIds.has(member.id));

  if (workspaceMembersQuery.isLoading || projectMembersQuery.isLoading) {
    return <FullPageMessage title="Загрузка участников проекта" embedded />;
  }

  if (projectMembersQuery.error || workspaceMembersQuery.error) {
    return (
      <FullPageMessage
        title="Не удалось загрузить участников проекта"
        description={getErrorMessage(projectMembersQuery.error ?? workspaceMembersQuery.error)}
        embedded
      />
    );
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!memberId) {
      return;
    }
    upsertMutation.mutate(
      { memberId, payload: { role } },
      {
        onSuccess: () => {
          setMemberId('');
          setRole('editor');
        },
      },
    );
  }

  return (
    <section className="directoryPage">
      <section className="directoryPanel">
        <div className="compactTitle">
          <Users size={16} />
          <h2>Участники проекта</h2>
        </div>
        {projectMembers.length > 0 ? (
          <div className="directoryList">
            {projectMembers.map((member) => (
              <div key={member.id} className="directoryRow directoryRowWithActions">
                <div className="directoryRowMain">
                  <Users size={18} />
                  <div>
                    <strong>{member.display_name}</strong>
                    <span>{member.github_login ?? member.external_subject}</span>
                  </div>
                  <span className={`statusPill status-${member.role}`}>{member.role === 'editor' ? 'редактор' : 'наблюдатель'}</span>
                </div>
                <div className="rowActions">
                  <select
                    value={member.role}
                    onChange={(event) =>
                      upsertMutation.mutate({
                        memberId: member.workspace_member_id,
                        payload: { role: event.target.value as 'editor' | 'viewer' },
                      })
                    }
                    disabled={upsertMutation.isPending}
                  >
                    <option value="editor">Редактор</option>
                    <option value="viewer">Наблюдатель</option>
                  </select>
                  <button
                    type="button"
                    className="iconButton dangerIconButton"
                    onClick={() => deleteMutation.mutate(member.workspace_member_id)}
                    disabled={deleteMutation.isPending}
                    title="Удалить доступ к проекту"
                    aria-label={`Удалить доступ к проекту для ${member.display_name}`}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">У проекта пока нет участников</div>
        )}
      </section>

      <section className="composePanel">
        <div className="compactTitle">
          <Plus size={16} />
          <h2>Выдать доступ</h2>
        </div>
        <form className="formGrid" onSubmit={handleSubmit}>
          <label className="field">
            <span>Участник</span>
            <select value={memberId} onChange={(event) => setMemberId(event.target.value)} required>
              <option value="">Выберите участника</option>
              {candidates.map((member) => (
                <option key={member.id} value={member.id}>
                  {member.display_name}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>Роль</span>
            <select value={role} onChange={(event) => setRole(event.target.value as 'editor' | 'viewer')}>
              <option value="editor">Редактор</option>
              <option value="viewer">Наблюдатель</option>
            </select>
          </label>
          <div className="formActions">
            <button type="submit" className="primaryButton compactButton" disabled={upsertMutation.isPending || !memberId}>
              <Plus size={16} />
              {upsertMutation.isPending ? 'Выдаем...' : 'Выдать'}
            </button>
          </div>
        </form>
        {upsertMutation.error || deleteMutation.error ? (
          <p className="errorText">{getErrorMessage(upsertMutation.error ?? deleteMutation.error)}</p>
        ) : null}
      </section>
    </section>
  );
}
