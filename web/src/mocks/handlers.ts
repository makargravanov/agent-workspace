import { http, HttpResponse } from 'msw';
import type { NoteDetail } from '../api/types';
import {
  mockAgents,
  mockAgentCredentials,
  mockNotes,
  mockProjects,
  mockSession,
  mockTasks,
  mockWorkspaces,
} from './data';

const BASE = '/api/v1';

let workspaceStore = [...mockWorkspaces];
let projectStore = [...mockProjects];
let agentStore = [...mockAgents];
let credentialStore = [...mockAgentCredentials];
let taskStore = [...mockTasks];
let noteStore = [...mockNotes];

function listEnvelope<T>(items: T[], requestId = 'mock-req') {
  return { data: { items, next_cursor: null }, meta: { request_id: requestId } };
}

function itemEnvelope<T>(data: T, requestId = 'mock-req') {
  return { data, meta: { request_id: requestId } };
}

function notFound(code: string, message: string) {
  return HttpResponse.json(
    { error: { code, message, details: null, request_id: 'mock-req' } },
    { status: 404 },
  );
}

export const handlers = [
  http.get(`${BASE}/auth/session`, () => HttpResponse.json(itemEnvelope(mockSession))),

  http.post(`${BASE}/auth/dev/login`, () => HttpResponse.json(itemEnvelope(mockSession))),

  http.post(`${BASE}/auth/logout`, () =>
    HttpResponse.json(itemEnvelope({ authenticated: false }), { status: 200 })),

  http.get(`${BASE}/workspaces`, () => HttpResponse.json(listEnvelope(workspaceStore))),

  http.get(`${BASE}/workspaces/:workspaceSlug`, ({ params }) => {
    const workspace = workspaceStore.find((item) => item.slug === params.workspaceSlug);
    return workspace
      ? HttpResponse.json(itemEnvelope(workspace))
      : notFound('workspace_not_found', 'Workspace not found');
  }),

  http.post(`${BASE}/workspaces`, async ({ request }) => {
    const body = (await request.json()) as { slug: string; name: string };
    const now = new Date().toISOString();
    const created = {
      id: crypto.randomUUID(),
      slug: body.slug,
      name: body.name,
      created_at: now,
      updated_at: now,
    };
    workspaceStore = [created, ...workspaceStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),

  http.get(`${BASE}/workspaces/:workspaceSlug/projects`, ({ params }) => {
    const workspace = workspaceStore.find((item) => item.slug === params.workspaceSlug);
    const items = workspace
      ? projectStore.filter((project) => project.workspace_id === workspace.id)
      : [];
    return HttpResponse.json(listEnvelope(items));
  }),

  http.get(`${BASE}/workspaces/:workspaceSlug/projects/:projectSlug`, ({ params }) => {
    const project = projectStore.find((item) => item.slug === params.projectSlug);
    return project
      ? HttpResponse.json(itemEnvelope(project))
      : notFound('project_not_found', 'Project not found');
  }),

  http.post(`${BASE}/workspaces/:workspaceSlug/projects`, async ({ request, params }) => {
    const workspace = workspaceStore.find((item) => item.slug === params.workspaceSlug);
    if (!workspace) {
      return notFound('workspace_not_found', 'Workspace not found');
    }
    const body = (await request.json()) as { slug: string; name: string };
    const now = new Date().toISOString();
    const created = {
      id: crypto.randomUUID(),
      workspace_id: workspace.id,
      slug: body.slug,
      name: body.name,
      status: 'active',
      created_at: now,
      updated_at: now,
    };
    projectStore = [created, ...projectStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),

  http.get(`${BASE}/workspaces/:workspaceSlug/agents`, ({ params }) => {
    const workspace = workspaceStore.find((item) => item.slug === params.workspaceSlug);
    const items = workspace ? agentStore.filter((agent) => agent.workspace_id === workspace.id) : [];
    return HttpResponse.json(listEnvelope(items));
  }),

  http.get(`${BASE}/workspaces/:workspaceSlug/agents/:agentId`, ({ params }) => {
    const agent = agentStore.find((item) => item.id === params.agentId);
    return agent ? HttpResponse.json(itemEnvelope(agent)) : notFound('agent_not_found', 'Agent not found');
  }),

  http.post(`${BASE}/workspaces/:workspaceSlug/agents`, async ({ request, params }) => {
    const workspace = workspaceStore.find((item) => item.slug === params.workspaceSlug);
    if (!workspace) return notFound('workspace_not_found', 'Workspace not found');
    const body = (await request.json()) as { key: string; display_name: string };
    const now = new Date().toISOString();
    const created = {
      id: crypto.randomUUID(),
      workspace_id: workspace.id,
      key: body.key,
      display_name: body.display_name,
      status: 'active' as const,
      created_at: now,
      updated_at: now,
    };
    agentStore = [created, ...agentStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),

  http.patch(`${BASE}/workspaces/:workspaceSlug/agents/:agentId`, async ({ request, params }) => {
    const index = agentStore.findIndex((item) => item.id === params.agentId);
    if (index === -1) return notFound('agent_not_found', 'Agent not found');
    const body = (await request.json()) as Partial<{ key: string; display_name: string; status: 'active' | 'disabled' }>;
    const updated = {
      ...agentStore[index],
      ...body,
      updated_at: new Date().toISOString(),
    };
    agentStore = agentStore.map((item, itemIndex) => (itemIndex === index ? updated : item));
    return HttpResponse.json(itemEnvelope(updated));
  }),

  http.delete(`${BASE}/workspaces/:workspaceSlug/agents/:agentId`, ({ params }) => {
    agentStore = agentStore.filter((agent) => agent.id !== params.agentId);
    credentialStore = credentialStore.filter((credential) => credential.agent_id !== params.agentId);
    return new HttpResponse(null, { status: 204 });
  }),

  http.get(`${BASE}/workspaces/:workspaceSlug/agents/:agentId/credentials`, ({ params }) => {
    const items = credentialStore.filter((credential) => credential.agent_id === params.agentId);
    return HttpResponse.json(listEnvelope(items));
  }),

  http.post(`${BASE}/workspaces/:workspaceSlug/agents/:agentId/credentials`, async ({ request, params }) => {
    const body = (await request.json()) as {
      label: string;
      project_id?: string | null;
      scopes?: string[];
      scope_policy?: string[];
      expires_at?: string | null;
    };
    const scopePolicy = body.scope_policy ?? body.scopes ?? [];
    const now = new Date().toISOString();
    const credential = {
      id: crypto.randomUUID(),
      workspace_id: '00000000-0000-0000-0000-000000000010',
      project_id: body.project_id ?? null,
      agent_id: params.agentId as string,
      label: body.label,
      secret_prefix: 'awsk_',
      scope_policy: scopePolicy,
      status: 'active' as const,
      expires_at: body.expires_at ?? null,
      created_at: now,
      updated_at: now,
    };
    credentialStore = [credential, ...credentialStore];
    return HttpResponse.json(itemEnvelope({ credential, secret: `secret-${credential.id.slice(0, 8)}` }), { status: 201 });
  }),

  http.get(`${BASE}/workspaces/:workspaceSlug/agent-credentials/:credentialId`, ({ params }) => {
    const credential = credentialStore.find((item) => item.id === params.credentialId);
    return credential ? HttpResponse.json(itemEnvelope(credential)) : notFound('agent_credential_not_found', 'Credential not found');
  }),

  http.patch(`${BASE}/workspaces/:workspaceSlug/agent-credentials/:credentialId`, async ({ request, params }) => {
    const index = credentialStore.findIndex((item) => item.id === params.credentialId);
    if (index === -1) return notFound('agent_credential_not_found', 'Credential not found');
    const body = (await request.json()) as Partial<{
      label: string;
      project_id: string | null;
      scopes: string[];
      scope_policy: string[];
      status: 'active' | 'revoked';
      expires_at: string | null;
    }>;
    const scopePolicy = body.scope_policy ?? body.scopes;
    const updated = {
      ...credentialStore[index],
      label: body.label ?? credentialStore[index].label,
      project_id: body.project_id ?? credentialStore[index].project_id,
      scope_policy: scopePolicy ?? credentialStore[index].scope_policy,
      status: body.status ?? credentialStore[index].status,
      expires_at: body.expires_at ?? credentialStore[index].expires_at,
      updated_at: new Date().toISOString(),
    };
    credentialStore = credentialStore.map((item, itemIndex) => (itemIndex === index ? updated : item));
    return HttpResponse.json(itemEnvelope(updated));
  }),

  http.delete(`${BASE}/workspaces/:workspaceSlug/agent-credentials/:credentialId`, ({ params }) => {
    credentialStore = credentialStore.filter((credential) => credential.id !== params.credentialId);
    return new HttpResponse(null, { status: 204 });
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/tasks`, ({ params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    const items = project ? taskStore.filter((task) => task.project_id === project.id) : [];
    return HttpResponse.json(listEnvelope(items));
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/tasks/:taskId`, ({ params }) => {
    const task = taskStore.find((item) => item.id === params.taskId);
    return task
      ? HttpResponse.json(itemEnvelope(task))
      : notFound('task_not_found', 'Task not found');
  }),

  http.post(`${BASE}/workspaces/:ws/projects/:proj/tasks`, async ({ request, params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    if (!project) {
      return notFound('project_not_found', 'Project not found');
    }
    const body = (await request.json()) as Record<string, unknown>;
    const now = new Date().toISOString();
    const created = {
      id: crypto.randomUUID(),
      project_id: project.id,
      group_id: (body.group_id as string | null) ?? null,
      parent_task_id: (body.parent_task_id as string | null) ?? null,
      title: body.title as string,
      description_md: (body.description_md as string | null) ?? null,
      status: 'todo' as const,
      priority: (body.priority as 'low' | 'normal' | 'high' | 'critical') ?? 'normal',
      rank_key: (body.rank_key as string) ?? 'a0',
      starts_at: null,
      due_at: null,
      assignee_type: (body.assignee_type as 'workspace_member' | 'agent' | null) ?? null,
      assignee_id: (body.assignee_id as string | null) ?? null,
      blocked: false,
      created_at: now,
      updated_at: now,
    };
    taskStore = [created, ...taskStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),

  http.patch(`${BASE}/workspaces/:ws/projects/:proj/tasks/:taskId/status`, async ({ request, params }) => {
    const body = (await request.json()) as { status: string };
    const index = taskStore.findIndex((item) => item.id === params.taskId);
    if (index === -1) {
      return notFound('task_not_found', 'Task not found');
    }
    const updated = {
      ...taskStore[index],
      status: body.status as 'todo' | 'in_progress' | 'done' | 'cancelled',
      updated_at: new Date().toISOString(),
    };
    taskStore = taskStore.map((item, itemIndex) => (itemIndex === index ? updated : item));
    return HttpResponse.json(itemEnvelope(updated));
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/notes`, ({ params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    const items = project ? noteStore.filter((note) => note.project_id === project.id) : [];
    return HttpResponse.json(listEnvelope(items));
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/notes/:noteId`, ({ params }) => {
    const note = noteStore.find((item) => item.id === params.noteId);
    return note
      ? HttpResponse.json(itemEnvelope(note))
      : notFound('note_not_found', 'Note not found');
  }),

  http.post(`${BASE}/workspaces/:ws/projects/:proj/notes`, async ({ request, params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    if (!project) {
      return notFound('project_not_found', 'Project not found');
    }
    const body = (await request.json()) as {
      kind: NoteDetail['kind'];
      title?: string | null;
      body_md: string;
    };
    const now = new Date().toISOString();
    const created = {
      id: crypto.randomUUID(),
      project_id: project.id,
      agent_session_id: null,
      kind: body.kind,
      author_type: 'workspace_member' as const,
      author_id: mockSession.actor?.actor_id ?? 'dev-user',
      title: body.title ?? null,
      body_md: body.body_md,
      created_at: now,
      updated_at: now,
    };
    noteStore = [created, ...noteStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),
];
