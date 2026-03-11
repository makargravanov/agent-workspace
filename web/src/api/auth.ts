import { apiGet, apiPost, type RequestOptions } from './client';
import type { ApiResponse, Session } from './types';

const path = {
  session: '/auth/session',
  githubStart: '/auth/github/start',
  devLogin: '/auth/dev/login',
  logout: '/auth/logout',
};

export async function getSession(opts?: RequestOptions): Promise<Session> {
  const resp = await apiGet<ApiResponse<Session>>(path.session, undefined, opts);
  return resp.data;
}

/**
 * Returns the browser URL to redirect to for GitHub OAuth flow.
 * The actual redirect is handled by the server; this endpoint is navigated to,
 * not called via fetch.
 */
export function getGithubStartUrl(): string {
  const base: string = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '/api/v1';
  return `${base}${path.githubStart}`;
}

/** Dev-only: log in as a preset user without OAuth. */
export async function devLogin(opts?: RequestOptions): Promise<Session> {
  const resp = await apiPost<Record<string, never>, ApiResponse<Session>>(
    path.devLogin,
    {},
    opts,
  );
  return resp.data;
}

export async function logout(opts?: RequestOptions): Promise<void> {
  await apiPost<Record<string, never>, unknown>(path.logout, {}, opts);
}
