import { http, HttpResponse } from 'msw';
import {
  mockNotes,
  mockProjects,
  mockSession,
  mockTaskGroups,
  mockTasks,
  mockWorkspaces,
} from './data';

const BASE = '/api/v1';

let taskStore = [...mockTasks];

/** Build a standard list envelope */
function listEnvelope<T>(items: T[], requestId = 'mock-req') {
  return { data: { items, next_cursor: null }, meta: { request_id: requestId } };
}

/** Build a standard single-item envelope */
function itemEnvelope<T>(data: T, requestId = 'mock-req') {
  return { data, meta: { request_id: requestId } };
}

function notFound(code: string, message: string) {
  return HttpResponse.json({ error: { code, message, details: null, request_id: 'mock-req' } }, { status: 404 });
}

export const handlers = [
  // ── Auth ──────────────────────────────────────────────────────────────────
  http.get(`${BASE}/auth/session`, () =>
    HttpResponse.json(itemEnvelope(mockSession)),
  ),

  http.post(`${BASE}/auth/dev/login`, () =>
    HttpResponse.json(itemEnvelope(mockSession)),
  ),

  http.post(`${BASE}/auth/logout`, () => new HttpResponse(null, { status: 204 })),

  // ── Workspaces ────────────────────────────────────────────────────────────
  http.get(`${BASE}/workspaces`, () =>
    HttpResponse.json(listEnvelope(mockWorkspaces)),
  ),

  http.get(`${BASE}/workspaces/:workspaceSlug`, ({ params }) => {
    const ws = mockWorkspaces.find((w) => w.slug === params.workspaceSlug);
    return ws ? HttpResponse.json(itemEnvelope(ws)) : notFound('workspace_not_found', 'Workspace not found');
  }),

  // ── Projects ──────────────────────────────────────────────────────────────
  http.get(`${BASE}/workspaces/:workspaceSlug/projects`, () =>
    HttpResponse.json(listEnvelope(mockProjects)),
  ),

  http.get(`${BASE}/workspaces/:workspaceSlug/projects/:projectSlug`, ({ params }) => {
    const proj = mockProjects.find((p) => p.slug === params.projectSlug);
    return proj ? HttpResponse.json(itemEnvelope(proj)) : notFound('project_not_found', 'Project not found');
  }),

  // ── Task groups ───────────────────────────────────────────────────────────
  http.get(`${BASE}/workspaces/:ws/projects/:proj/task-groups`, () =>
    HttpResponse.json(listEnvelope(mockTaskGroups)),
  ),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/task-groups/:groupId`, ({ params }) => {
    const group = mockTaskGroups.find((g) => g.id === params.groupId);
    return group ? HttpResponse.json(itemEnvelope(group)) : notFound('task_group_not_found', 'Task group not found');
  }),

  // ── Tasks ─────────────────────────────────────────────────────────────────
  http.get(`${BASE}/workspaces/:ws/projects/:proj/tasks`, () =>
    HttpResponse.json(listEnvelope(taskStore)),
  ),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/tasks/:taskId`, ({ params }) => {
    const task = taskStore.find((t) => t.id === params.taskId);
    return task ? HttpResponse.json(itemEnvelope(task)) : notFound('task_not_found', 'Task not found');
  }),

  http.post(`${BASE}/workspaces/:ws/projects/:proj/tasks`, async ({ request, params }) => {
    const body = await request.json() as Record<string, unknown>;
    const now = new Date().toISOString();
    const newTask = {
      id: crypto.randomUUID(),
      project_id: mockProjects.find((p) => p.slug === params.proj)?.id ?? '',
      group_id: (body.group_id as string | null) ?? null,
      parent_task_id: (body.parent_task_id as string | null) ?? null,
      title: body.title as string,
      description_md: (body.description_md as string | null) ?? null,
      status: 'todo' as const,
      priority: (body.priority as 'low' | 'medium' | 'high' | 'critical') ?? 'medium',
      rank_key: (body.rank_key as string) ?? 'a0',
      starts_at: null,
      due_at: null,
      assignee_type: (body.assignee_type as 'workspace_member' | 'agent' | null) ?? null,
      assignee_id: (body.assignee_id as string | null) ?? null,
      blocked: false,
      created_at: now,
      updated_at: now,
    };
    taskStore = [...taskStore, newTask];
    return HttpResponse.json(itemEnvelope(newTask), { status: 201 });
  }),

  http.patch(`${BASE}/workspaces/:ws/projects/:proj/tasks/:taskId/status`, async ({ request, params }) => {
    const body = await request.json() as { status: string };
    const idx = taskStore.findIndex((t) => t.id === params.taskId);
    if (idx === -1) return notFound('task_not_found', 'Task not found');
    const updated = { ...taskStore[idx], status: body.status as 'todo' | 'in_progress' | 'done' | 'cancelled', updated_at: new Date().toISOString() };
    taskStore = taskStore.map((t, i) => (i === idx ? updated : t));
    return HttpResponse.json(itemEnvelope(updated));
  }),

  // ── Notes ─────────────────────────────────────────────────────────────────
  http.get(`${BASE}/workspaces/:ws/projects/:proj/notes`, () =>
    HttpResponse.json(listEnvelope(mockNotes)),
  ),

  http.get(`${BASE}/workspaces/:ws/projects/:proj/notes/:noteId`, ({ params }) => {
    const note = mockNotes.find((n) => n.id === params.noteId);
    return note ? HttpResponse.json(itemEnvelope(note)) : notFound('note_not_found', 'Note not found');
  }),
];
