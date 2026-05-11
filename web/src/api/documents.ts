import { apiDelete, apiGet, apiPatch, apiPost, type RequestOptions } from './client';
import type {
  ApiListResponse,
  ApiResponse,
  CreateDocumentPayload,
  DocumentDetail,
  PaginationParams,
  RepairDocumentCyclesResult,
  UpdateDocumentPayload,
} from './types';

const documentsBase = (workspaceSlug: string, projectSlug: string) =>
  `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`;

export async function listDocuments(
  workspaceSlug: string,
  projectSlug: string,
  pagination?: PaginationParams,
  opts?: RequestOptions,
): Promise<ApiListResponse<DocumentDetail>['data']> {
  const resp = await apiGet<ApiListResponse<DocumentDetail>>(
    documentsBase(workspaceSlug, projectSlug),
    pagination,
    opts,
  );
  return resp.data;
}

export async function getDocument(
  workspaceSlug: string,
  projectSlug: string,
  documentId: string,
  opts?: RequestOptions,
): Promise<DocumentDetail> {
  const resp = await apiGet<ApiResponse<DocumentDetail>>(
    `${documentsBase(workspaceSlug, projectSlug)}/${documentId}`,
    undefined,
    opts,
  );
  return resp.data;
}

export async function createDocument(
  workspaceSlug: string,
  projectSlug: string,
  payload: CreateDocumentPayload,
  opts?: RequestOptions,
): Promise<DocumentDetail> {
  const resp = await apiPost<CreateDocumentPayload, ApiResponse<DocumentDetail>>(
    documentsBase(workspaceSlug, projectSlug),
    payload,
    opts,
  );
  return resp.data;
}

export async function updateDocument(
  workspaceSlug: string,
  projectSlug: string,
  documentId: string,
  payload: UpdateDocumentPayload,
  opts?: RequestOptions,
): Promise<DocumentDetail> {
  const resp = await apiPatch<UpdateDocumentPayload, ApiResponse<DocumentDetail>>(
    `${documentsBase(workspaceSlug, projectSlug)}/${documentId}`,
    payload,
    opts,
  );
  return resp.data;
}

export async function deleteDocument(
  workspaceSlug: string,
  projectSlug: string,
  documentId: string,
  opts?: RequestOptions,
): Promise<void> {
  await apiDelete(`${documentsBase(workspaceSlug, projectSlug)}/${documentId}`, opts);
}

export async function repairDocumentCycles(
  workspaceSlug: string,
  projectSlug: string,
  opts?: RequestOptions,
): Promise<RepairDocumentCyclesResult> {
  const resp = await apiPost<Record<string, never>, ApiResponse<RepairDocumentCyclesResult>>(
    `${documentsBase(workspaceSlug, projectSlug)}/repair-cycles`,
    {},
    opts,
  );
  return resp.data;
}
