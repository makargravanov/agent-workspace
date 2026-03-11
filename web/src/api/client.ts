import type { ApiErrorBody } from './types';

// ─── Base URL ─────────────────────────────────────────────────────────────────

const BASE_URL: string = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '/api/v1';

// ─── ApiError ─────────────────────────────────────────────────────────────────

/**
 * Typed error thrown for any non-2xx response.
 * Carries the server error code, request_id, and HTTP status for diagnostic use.
 */
export class ApiError extends Error {
  readonly code: string;
  readonly requestId: string;
  readonly details: unknown;
  readonly statusCode: number;

  constructor(statusCode: number, body: ApiErrorBody) {
    super(body.message);
    this.name = 'ApiError';
    this.code = body.code;
    this.requestId = body.request_id;
    this.details = body.details;
    this.statusCode = statusCode;
  }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

function buildUrl(
  path: string,
  params?: Record<string, string | number | undefined>,
): string {
  const base = `${BASE_URL}${path}`;
  if (!params) return base;
  const entries = Object.entries(params).filter(([, v]) => v !== undefined);
  if (entries.length === 0) return base;
  const query = entries
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
    .join('&');
  return `${base}?${query}`;
}

async function parseResponse<T>(res: Response): Promise<T> {
  const json: unknown = await res.json();
  if (!res.ok) {
    const errBody = (json as { error: ApiErrorBody }).error;
    throw new ApiError(res.status, errBody);
  }
  return json as T;
}

// ─── Request options ──────────────────────────────────────────────────────────

export interface RequestOptions {
  signal?: AbortSignal;
}

// ─── HTTP verbs ───────────────────────────────────────────────────────────────

/**
 * Typed GET. `T` should be the full envelope, e.g. `ApiResponse<WorkspaceSummary>`.
 * The caller (endpoint module) is responsible for unwrapping `.data`.
 */
export async function apiGet<T>(
  path: string,
  params?: Record<string, string | number | undefined>,
  opts?: RequestOptions,
): Promise<T> {
  const res = await fetch(buildUrl(path, params), {
    method: 'GET',
    credentials: 'include',
    signal: opts?.signal,
  });
  return parseResponse<T>(res);
}

/**
 * Typed POST. Serializes `body` as JSON.
 */
export async function apiPost<TBody, TResponse>(
  path: string,
  body: TBody,
  opts?: RequestOptions,
): Promise<TResponse> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
    signal: opts?.signal,
  });
  return parseResponse<TResponse>(res);
}

/**
 * Typed PATCH. Serializes `body` as JSON.
 */
export async function apiPatch<TBody, TResponse>(
  path: string,
  body: TBody,
  opts?: RequestOptions,
): Promise<TResponse> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'PATCH',
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
    signal: opts?.signal,
  });
  return parseResponse<TResponse>(res);
}

/**
 * Typed DELETE. Returns void on success.
 */
export async function apiDelete(path: string, opts?: RequestOptions): Promise<void> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'DELETE',
    credentials: 'include',
    signal: opts?.signal,
  });
  if (!res.ok) {
    const json: unknown = await res.json();
    throw new ApiError(res.status, (json as { error: ApiErrorBody }).error);
  }
}
