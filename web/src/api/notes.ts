import { apiGet, apiPost, type RequestOptions } from './client';
import type {
  ApiListResponse,
  ApiResponse,
  CreateNotePayload,
  NoteDetail,
  PaginationParams,
} from './types';

const notesBase = (ws: string, proj: string) =>
  `/workspaces/${ws}/projects/${proj}/notes`;

export async function listNotes(
  workspaceSlug: string,
  projectSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<NoteDetail>['data']> {
  const resp = await apiGet<ApiListResponse<NoteDetail>>(
    notesBase(workspaceSlug, projectSlug),
    pagination,
    opts,
  );
  return resp.data;
}

export async function getNote(
  workspaceSlug: string,
  projectSlug: string,
  noteId: string,
  opts?: RequestOptions,
): Promise<NoteDetail> {
  const resp = await apiGet<ApiResponse<NoteDetail>>(
    `${notesBase(workspaceSlug, projectSlug)}/${noteId}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createNote(
  workspaceSlug: string,
  projectSlug: string,
  payload: CreateNotePayload,
  opts?: RequestOptions,
): Promise<NoteDetail> {
  const resp = await apiPost<CreateNotePayload, ApiResponse<NoteDetail>>(
    notesBase(workspaceSlug, projectSlug),
    payload,
    opts,
  );
  return resp.data;
}
