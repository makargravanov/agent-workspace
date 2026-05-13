import { http, HttpResponse } from 'msw';
import type { AssetDetail, DocumentDetail, NoteDetail } from '../api/types';
import {
  mockAssets,
  mockAgents,
  mockAgentCredentials,
  mockDocuments,
  mockIntegrationConnections,
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
let integrationConnectionStore = [...mockIntegrationConnections];
let taskStore = [...mockTasks];
let noteStore = [...mockNotes];
let documentStore = [...mockDocuments];
let assetStore = [...mockAssets];
const assetContentStore = new Map<string, string>(
  mockAssets.map((asset) => [asset.id, btoa(`Mock content for ${asset.file_name}`)]),
);

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

  http.get(`${BASE}/workspaces/:workspaceSlug/integration-connections`, ({ params }) => {
    const workspace = workspaceStore.find((item) => item.slug === params.workspaceSlug);
    const items = workspace
      ? integrationConnectionStore.filter((connection) => connection.workspace_id === workspace.id)
      : [];
    return HttpResponse.json(listEnvelope(items));
  }),

  http.post(`${BASE}/workspaces/:workspaceSlug/integration-connections`, async ({ request, params }) => {
    const workspace = workspaceStore.find((item) => item.slug === params.workspaceSlug);
    if (!workspace) return notFound('workspace_not_found', 'Workspace not found');
    const body = (await request.json()) as {
      provider?: string;
      scope_kind: 'workspace' | 'project';
      project_id?: string | null;
      status?: 'active' | 'disabled' | 'error';
      config_json?: unknown;
    };
    if (body.scope_kind === 'project' && !body.project_id) {
      return HttpResponse.json(
        {
          error: {
            code: 'validation_error',
            message: 'project_id is required when scope_kind = project',
            details: null,
            request_id: 'mock-req',
          },
        },
        { status: 422 },
      );
    }
    const now = new Date().toISOString();
    const created = {
      id: crypto.randomUUID(),
      workspace_id: workspace.id,
      project_id: body.scope_kind === 'project' ? body.project_id ?? null : null,
      provider: body.provider ?? 'github',
      scope_kind: body.scope_kind,
      status: body.status ?? 'active',
      config_json:
        body.config_json === null || body.config_json === undefined
          ? null
          : JSON.stringify(body.config_json),
      secret_ciphertext: null,
      last_synced_at: null,
      created_at: now,
      updated_at: now,
    };
    integrationConnectionStore = [created, ...integrationConnectionStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),

  http.get(`${BASE}/workspaces/:workspaceSlug/integration-connections/:connectionId`, ({ params }) => {
    const connection = integrationConnectionStore.find((item) => item.id === params.connectionId);
    return connection
      ? HttpResponse.json(itemEnvelope(connection))
      : notFound('integration_connection_not_found', 'Connection not found');
  }),

  http.patch(`${BASE}/workspaces/:workspaceSlug/integration-connections/:connectionId`, async ({ request, params }) => {
    const index = integrationConnectionStore.findIndex((item) => item.id === params.connectionId);
    if (index === -1) {
      return notFound('integration_connection_not_found', 'Connection not found');
    }
    const body = (await request.json()) as Partial<{
      status: 'active' | 'disabled' | 'error';
      config_json: unknown;
    }>;
    const updated = {
      ...integrationConnectionStore[index],
      status: body.status ?? integrationConnectionStore[index].status,
      config_json:
        body.config_json === undefined
          ? integrationConnectionStore[index].config_json
          : body.config_json === null
            ? null
            : JSON.stringify(body.config_json),
      updated_at: new Date().toISOString(),
    };
    integrationConnectionStore = integrationConnectionStore.map((item, itemIndex) =>
      itemIndex === index ? updated : item,
    );
    return HttpResponse.json(itemEnvelope(updated));
  }),

  http.delete(`${BASE}/workspaces/:workspaceSlug/integration-connections/:connectionId`, ({ params }) => {
    integrationConnectionStore = integrationConnectionStore.filter(
      (connection) => connection.id !== params.connectionId,
    );
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

  http.get(`${BASE}/workspaces/:ws/projects/:proj/documents`, ({ params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    const items = project ? documentStore.filter((document) => document.project_id === project.id) : [];
    return HttpResponse.json(listEnvelope(items));
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/documents/:documentId`, ({ params }) => {
    const document = documentStore.find((item) => item.id === params.documentId);
    return document
      ? HttpResponse.json(itemEnvelope(document))
      : notFound('document_not_found', 'Document not found');
  }),

  http.post(`${BASE}/workspaces/:ws/projects/:proj/documents`, async ({ request, params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    if (!project) {
      return notFound('project_not_found', 'Project not found');
    }
    const body = (await request.json()) as Partial<{
      slug: string;
      title: string;
      body_md: string;
      parent_document_id: string | null;
      body_format: string;
      status: DocumentDetail['status'];
    }>;
    const now = new Date().toISOString();
    const created = {
      id: crypto.randomUUID(),
      workspace_id: project.workspace_id,
      project_id: project.id,
      parent_document_id: body.parent_document_id ?? null,
      slug: body.slug ?? 'untitled',
      title: body.title ?? 'Untitled',
      body_format: body.body_format ?? 'markdown',
      body_md: body.body_md ?? '',
      status: body.status ?? 'draft',
      version: 1,
      created_at: now,
      updated_at: now,
    };
    documentStore = [created, ...documentStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),

  http.patch(`${BASE}/workspaces/:ws/projects/:proj/documents/:documentId`, async ({ request, params }) => {
    const index = documentStore.findIndex((item) => item.id === params.documentId);
    if (index === -1) {
      return notFound('document_not_found', 'Document not found');
    }
    const body = (await request.json()) as Partial<{
      version: number;
      slug: string;
      title: string;
      body_md: string;
      parent_document_id: string | null;
      body_format: string;
      status: DocumentDetail['status'];
    }>;
    const current = documentStore[index];
    if (body.version !== current.version) {
      return HttpResponse.json(
        {
          error: {
            code: 'conflict',
            message: 'document version is stale; reload before updating',
            details: null,
            request_id: 'mock-req',
          },
        },
        { status: 409 },
      );
    }
    const updated = {
      ...current,
      slug: body.slug ?? current.slug,
      title: body.title ?? current.title,
      body_md: body.body_md ?? current.body_md,
      body_format: body.body_format ?? current.body_format,
      status: body.status ?? current.status,
      parent_document_id:
        body.parent_document_id === undefined ? current.parent_document_id : body.parent_document_id,
      version: current.version + 1,
      updated_at: new Date().toISOString(),
    };
    documentStore = documentStore.map((item, itemIndex) => (itemIndex === index ? updated : item));
    return HttpResponse.json(itemEnvelope(updated));
  }),

  http.delete(`${BASE}/workspaces/:ws/projects/:proj/documents/:documentId`, ({ params }) => {
    documentStore = documentStore.filter((document) => document.id !== params.documentId);
    documentStore = documentStore.map((document) =>
      document.parent_document_id === params.documentId
        ? { ...document, parent_document_id: null }
        : document,
    );
    return new HttpResponse(null, { status: 204 });
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/assets`, ({ params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    const items = project ? assetStore.filter((asset) => asset.project_id === project.id) : [];
    return HttpResponse.json(listEnvelope(items));
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/assets/:assetId`, ({ params }) => {
    const asset = assetStore.find((item) => item.id === params.assetId);
    return asset ? HttpResponse.json(itemEnvelope(asset)) : notFound('asset_not_found', 'Asset not found');
  }),

  http.post(`${BASE}/workspaces/:ws/projects/:proj/assets`, async ({ request, params }) => {
    const project = projectStore.find((item) => item.slug === params.proj);
    if (!project) {
      return notFound('project_not_found', 'Project not found');
    }
    const body = (await request.json()) as {
      file_name: string;
      media_type: string;
      content_base64: string;
      sha256?: string | null;
    };
    if (!body.content_base64 || !body.file_name?.trim() || !body.media_type?.trim()) {
      return HttpResponse.json(
        {
          error: {
            code: 'validation_error',
            message: 'file_name, media_type and content_base64 are required',
            details: null,
            request_id: 'mock-req',
          },
        },
        { status: 422 },
      );
    }
    const now = new Date().toISOString();
    const id = crypto.randomUUID();
    const size = base64Size(body.content_base64);
    const created: AssetDetail = {
      id,
      workspace_id: project.workspace_id,
      project_id: project.id,
      uploaded_by_member_id: mockSession.actor?.actor_id ?? null,
      uploaded_by_github_login: null,
      file_name: body.file_name,
      media_type: body.media_type,
      size_bytes: size,
      sha256: body.sha256 ?? null,
      storage_backend: 'local',
      storage_key: id,
      created_at: now,
    };
    assetContentStore.set(id, body.content_base64);
    assetStore = [created, ...assetStore];
    return HttpResponse.json(itemEnvelope(created), { status: 201 });
  }),

  http.patch(`${BASE}/workspaces/:ws/projects/:proj/assets/:assetId`, async ({ request, params }) => {
    const index = assetStore.findIndex((item) => item.id === params.assetId);
    if (index === -1) {
      return notFound('asset_not_found', 'Asset not found');
    }
    const body = (await request.json()) as Partial<{
      file_name: string;
      media_type: string;
      content_base64: string;
      sha256: string | null;
    }>;
    const current = assetStore[index];
    const updated = {
      ...current,
      file_name: body.file_name ?? current.file_name,
      media_type: body.media_type ?? current.media_type,
      size_bytes: body.content_base64 ? base64Size(body.content_base64) : current.size_bytes,
      sha256: body.sha256 === undefined ? current.sha256 : body.sha256,
    };
    if (body.content_base64) {
      assetContentStore.set(current.id, body.content_base64);
    }
    assetStore = assetStore.map((item, itemIndex) => (itemIndex === index ? updated : item));
    return HttpResponse.json(itemEnvelope(updated));
  }),

  http.delete(`${BASE}/workspaces/:ws/projects/:proj/assets/:assetId`, ({ params }) => {
    assetStore = assetStore.filter((asset) => asset.id !== params.assetId);
    assetContentStore.delete(params.assetId as string);
    return new HttpResponse(null, { status: 204 });
  }),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/assets/:assetId/download`, ({ params, request }) => {
    const asset = assetStore.find((item) => item.id === params.assetId);
    if (!asset) {
      return notFound('asset_not_found', 'Asset not found');
    }
    const contentBase64 = assetContentStore.get(asset.id) ?? '';
    const bytes = Uint8Array.from(atob(contentBase64), (char) => char.charCodeAt(0));
    const disposition = new URL(request.url).searchParams.get('disposition') === 'inline'
      ? 'inline'
      : 'attachment';
    return new HttpResponse(bytes, {
      status: 200,
      headers: {
        'Content-Type': asset.media_type,
        'Content-Disposition': `${disposition}; filename="${asset.file_name}"`,
      },
    });
  }),
];

function base64Size(contentBase64: string): number {
  try {
    return atob(contentBase64).length;
  } catch {
    return 0;
  }
}
