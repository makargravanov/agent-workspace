import { Link2, Plus, Trash2, UserPlus, Users } from 'lucide-react';
import type { FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import type { AccessRole, WorkspaceMember } from '../../api/types';
import {
  useCreateWorkspaceInvite,
  useDeleteWorkspaceInvite,
  useUpdateWorkspaceMember,
  useWorkspaceInvites,
  useWorkspaceMembers,
} from '../../hooks/useMembers';
import { useProjects } from '../../hooks/useWorkspaces';
import { getErrorMessage } from '../../shared/lib/errors';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

export function WorkspaceMembersPage() {
  const { workspaceSlug = '' } = useParams();
  const membersQuery = useWorkspaceMembers(workspaceSlug);
  const invitesQuery = useWorkspaceInvites(workspaceSlug);
  const projectsQuery = useProjects(workspaceSlug);
  const createInviteMutation = useCreateWorkspaceInvite(workspaceSlug);
  const deleteInviteMutation = useDeleteWorkspaceInvite(workspaceSlug);
  const updateMemberMutation = useUpdateWorkspaceMember(workspaceSlug);
  const [githubLogin, setGithubLogin] = useState('');
  const [role, setRole] = useState<'editor' | 'viewer'>('editor');
  const [projectRole, setProjectRole] = useState<'editor' | 'viewer'>('editor');
  const [selectedProjectIds, setSelectedProjectIds] = useState<string[]>([]);

  const members = useMemo(() => membersQuery.data?.items ?? [], [membersQuery.data?.items]);
  const invites = useMemo(() => invitesQuery.data?.items ?? [], [invitesQuery.data?.items]);
  const projects = useMemo(() => projectsQuery.data?.items ?? [], [projectsQuery.data?.items]);

  if (membersQuery.isLoading || invitesQuery.isLoading) {
    return <FullPageMessage title="Загрузка участников" embedded />;
  }

  if (membersQuery.error) {
    return (
      <FullPageMessage
        title="Не удалось загрузить участников"
        description={getErrorMessage(membersQuery.error)}
        embedded
      />
    );
  }

  function handleInvite(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createInviteMutation.mutate(
      {
        github_login: githubLogin.trim() || undefined,
        role,
        project_access: selectedProjectIds.map((projectId) => ({
          project_id: projectId,
          role: projectRole,
        })),
      },
      {
        onSuccess: () => {
          setGithubLogin('');
          setSelectedProjectIds([]);
        },
      },
    );
  }

  function toggleProject(projectId: string) {
    setSelectedProjectIds((current) =>
      current.includes(projectId)
        ? current.filter((id) => id !== projectId)
        : [...current, projectId],
    );
  }

  return (
    <section className="directoryPage">
      <section className="directoryPanel">
        <div className="compactTitle">
          <Users size={16} />
          <h2>Участники рабочего пространства</h2>
        </div>
        {members.length > 0 ? (
          <div className="directoryList">
            {members.map((member) => (
              <MemberRow
                key={member.id}
                member={member}
                onUpdate={(payload) =>
                  updateMemberMutation.mutate({ memberId: member.id, payload })
                }
                disabled={updateMemberMutation.isPending}
              />
            ))}
          </div>
        ) : (
          <div className="emptyPanel">Участников пока нет</div>
        )}
      </section>

      <section className="composePanel">
        <div className="compactTitle">
          <UserPlus size={16} />
          <h2>Пригласить участника</h2>
        </div>
        <form className="formGrid" onSubmit={handleInvite}>
          <label className="field">
            <span>GitHub login</span>
            <input
              value={githubLogin}
              onChange={(event) => setGithubLogin(event.target.value)}
              placeholder="octocat"
            />
          </label>
          <label className="field">
            <span>Роль в рабочем пространстве</span>
            <select value={role} onChange={(event) => setRole(event.target.value as 'editor' | 'viewer')}>
              <option value="editor">Редактор</option>
              <option value="viewer">Наблюдатель</option>
            </select>
          </label>
          <label className="field">
            <span>Роль в проектах</span>
            <select value={projectRole} onChange={(event) => setProjectRole(event.target.value as 'editor' | 'viewer')}>
              <option value="editor">Редактор</option>
              <option value="viewer">Наблюдатель</option>
            </select>
          </label>
          <div className="field">
            <span>Начальные проекты</span>
            <div className="checkboxStack">
              {projects.map((project) => (
                <label key={project.id} className="checkboxLine">
                  <input
                    type="checkbox"
                    checked={selectedProjectIds.includes(project.id)}
                    onChange={() => toggleProject(project.id)}
                  />
                  <span>{project.name}</span>
                </label>
              ))}
            </div>
          </div>
          <div className="formActions">
            <button type="submit" className="primaryButton compactButton" disabled={createInviteMutation.isPending}>
              <Plus size={16} />
              {createInviteMutation.isPending ? 'Создание...' : 'Создать приглашение'}
            </button>
          </div>
        </form>
        {createInviteMutation.error || deleteInviteMutation.error ? (
          <p className="errorText">{getErrorMessage(createInviteMutation.error ?? deleteInviteMutation.error)}</p>
        ) : null}
      </section>

      <section className="directoryPanel">
        <div className="compactTitle">
          <UserPlus size={16} />
          <h2>Приглашения</h2>
        </div>
        {invites.length > 0 ? (
          <div className="directoryList">
            {invites.map((invite) => (
              <div key={invite.id} className="directoryRow directoryRowWithActions">
                <div className="directoryRowMain">
                  <UserPlus size={18} />
                  <div>
                    <strong>{invite.github_login ?? 'Ссылка-приглашение'}</strong>
                    <span>{formatInviteRole(invite.role)} / {formatInviteStatus(invite.status)}</span>
                    {invite.invite_url ? (
                      <a href={invite.invite_url} target="_blank" rel="noreferrer">
                        {toAbsoluteInviteUrl(invite.invite_url)}
                      </a>
                    ) : null}
                  </div>
                </div>
                <div className="rowActions">
                  <button
                    type="button"
                    className="iconButton"
                    onClick={() => {
                      if (invite.invite_url) {
                        void navigator.clipboard.writeText(toAbsoluteInviteUrl(invite.invite_url));
                      }
                    }}
                    title="Скопировать ссылку"
                    aria-label={`Скопировать ссылку приглашения ${invite.github_login ?? invite.id}`}
                    disabled={!invite.invite_url}
                  >
                    <Link2 size={16} />
                  </button>
                  <button
                    type="button"
                    className="iconButton dangerIconButton"
                    onClick={() => deleteInviteMutation.mutate(invite.id)}
                    disabled={deleteInviteMutation.isPending || invite.status !== 'pending'}
                    title="Удалить приглашение"
                    aria-label={`Удалить приглашение ${invite.github_login ?? invite.id}`}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">Приглашений пока нет</div>
        )}
      </section>
    </section>
  );
}

function MemberRow({
  member,
  onUpdate,
  disabled,
}: {
  member: WorkspaceMember;
  onUpdate: (payload: { role?: AccessRole; status?: 'active' | 'disabled' }) => void;
  disabled: boolean;
}) {
  return (
    <div className="directoryRow directoryRowWithActions">
      <div className="directoryRowMain">
        <Users size={18} />
        <div>
          <strong>{member.display_name}</strong>
          <span>{member.github_login ?? member.external_subject}</span>
        </div>
        <span className={`statusPill status-${member.status}`}>{formatMemberStatus(member.status)}</span>
      </div>
      <div className="rowActions">
        <select
          value={member.role}
          onChange={(event) => onUpdate({ role: event.target.value as AccessRole })}
          disabled={disabled}
        >
          <option value="owner">Владелец</option>
          <option value="editor">Редактор</option>
          <option value="viewer">Наблюдатель</option>
        </select>
        <select
          value={member.status}
          onChange={(event) => onUpdate({ status: event.target.value as 'active' | 'disabled' })}
          disabled={disabled}
        >
          <option value="active">Активен</option>
          <option value="disabled">Отключен</option>
        </select>
      </div>
    </div>
  );
}

function formatInviteRole(role: 'editor' | 'viewer') {
  return role === 'editor' ? 'редактор' : 'наблюдатель';
}

function formatInviteStatus(status: 'pending' | 'accepted' | 'revoked' | 'expired') {
  switch (status) {
    case 'accepted':
      return 'принято';
    case 'revoked':
      return 'удалено';
    case 'expired':
      return 'истекло';
    default:
      return 'ожидает';
  }
}

function formatMemberStatus(status: 'active' | 'invited' | 'disabled') {
  switch (status) {
    case 'disabled':
      return 'отключен';
    case 'invited':
      return 'приглашен';
    default:
      return 'активен';
  }
}

function toAbsoluteInviteUrl(inviteUrl: string) {
  return new URL(inviteUrl, window.location.origin).toString();
}
