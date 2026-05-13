import { Plus, UserPlus, Users } from 'lucide-react';
import type { FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import type { AccessRole, WorkspaceMember } from '../../api/types';
import {
  useCreateWorkspaceInvite,
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
  const updateMemberMutation = useUpdateWorkspaceMember(workspaceSlug);
  const [githubLogin, setGithubLogin] = useState('');
  const [role, setRole] = useState<'editor' | 'viewer'>('editor');
  const [projectRole, setProjectRole] = useState<'editor' | 'viewer'>('editor');
  const [selectedProjectIds, setSelectedProjectIds] = useState<string[]>([]);

  const members = useMemo(() => membersQuery.data?.items ?? [], [membersQuery.data?.items]);
  const invites = useMemo(() => invitesQuery.data?.items ?? [], [invitesQuery.data?.items]);
  const projects = useMemo(() => projectsQuery.data?.items ?? [], [projectsQuery.data?.items]);

  if (membersQuery.isLoading || invitesQuery.isLoading) {
    return <FullPageMessage title="Loading members" embedded />;
  }

  if (membersQuery.error) {
    return (
      <FullPageMessage
        title="Members unavailable"
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
          <h2>Workspace members</h2>
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
          <div className="emptyPanel">No members</div>
        )}
      </section>

      <section className="composePanel">
        <div className="compactTitle">
          <UserPlus size={16} />
          <h2>Invite developer</h2>
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
            <span>Workspace role</span>
            <select value={role} onChange={(event) => setRole(event.target.value as 'editor' | 'viewer')}>
              <option value="editor">editor</option>
              <option value="viewer">viewer</option>
            </select>
          </label>
          <label className="field">
            <span>Project role</span>
            <select value={projectRole} onChange={(event) => setProjectRole(event.target.value as 'editor' | 'viewer')}>
              <option value="editor">editor</option>
              <option value="viewer">viewer</option>
            </select>
          </label>
          <div className="field">
            <span>Initial projects</span>
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
              {createInviteMutation.isPending ? 'Creating...' : 'Create invite'}
            </button>
          </div>
        </form>
        {createInviteMutation.error ? (
          <p className="errorText">{getErrorMessage(createInviteMutation.error)}</p>
        ) : null}
      </section>

      <section className="directoryPanel">
        <div className="compactTitle">
          <UserPlus size={16} />
          <h2>Invites</h2>
        </div>
        {invites.length > 0 ? (
          <div className="directoryList">
            {invites.map((invite) => (
              <div key={invite.id} className="directoryRow">
                <div className="directoryRowMain">
                  <UserPlus size={18} />
                  <div>
                    <strong>{invite.github_login ?? 'Invite link'}</strong>
                    <span>{invite.role} / {invite.status}</span>
                    {invite.invite_url ? <span>{invite.invite_url}</span> : null}
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="emptyPanel">No invites</div>
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
        <span className={`statusPill status-${member.status}`}>{member.status}</span>
      </div>
      <div className="rowActions">
        <select
          value={member.role}
          onChange={(event) => onUpdate({ role: event.target.value as AccessRole })}
          disabled={disabled}
        >
          <option value="owner">owner</option>
          <option value="editor">editor</option>
          <option value="viewer">viewer</option>
        </select>
        <select
          value={member.status}
          onChange={(event) => onUpdate({ status: event.target.value as 'active' | 'disabled' })}
          disabled={disabled}
        >
          <option value="active">active</option>
          <option value="disabled">disabled</option>
        </select>
      </div>
    </div>
  );
}
