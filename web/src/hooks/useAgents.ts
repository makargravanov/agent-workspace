import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import {
  createAgent,
  createAgentCredential,
  deleteAgent,
  deleteAgentCredential,
  getAgent,
  getAgentCredential,
  listAgentCredentials,
  listAgents,
  updateAgent,
  updateAgentCredential,
} from '../api/agents';
import { listProjects } from '../api/workspaces';
import type {
  AgentCredentialSummary,
  AgentSummary,
  ApiListData,
  CreateAgentCredentialPayload,
  CreateAgentPayload,
  CredentialStatus,
  UpdateAgentCredentialPayload,
  UpdateAgentPayload,
} from '../api/types';

export function useAgents(workspaceSlug: string) {
  return useQuery({
    queryKey: queryKeys.agents(workspaceSlug),
    queryFn: ({ signal }) => listAgents(workspaceSlug, undefined, { signal }),
    enabled: workspaceSlug.length > 0,
  });
}

export function useAgent(workspaceSlug: string, agentId: string) {
  return useQuery({
    queryKey: queryKeys.agent(workspaceSlug, agentId),
    queryFn: ({ signal }) => getAgent(workspaceSlug, agentId, { signal }),
    enabled: workspaceSlug.length > 0 && agentId.length > 0,
  });
}

export function useCreateAgent(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateAgentPayload) => createAgent(workspaceSlug, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(queryKeys.agent(workspaceSlug, created.id), created);
      void queryClient.invalidateQueries({ queryKey: queryKeys.agents(workspaceSlug) });
    },
  });
}

export function useUpdateAgent(workspaceSlug: string, agentId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateAgentPayload) => updateAgent(workspaceSlug, agentId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.agent(workspaceSlug, agentId), updated);
      void queryClient.invalidateQueries({ queryKey: queryKeys.agents(workspaceSlug) });
    },
  });
}

export function useDeleteAgent(workspaceSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (agentId: string) => deleteAgent(workspaceSlug, agentId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.agents(workspaceSlug) });
    },
  });
}

export function useAgentCredentials(workspaceSlug: string, agentId: string) {
  return useQuery({
    queryKey: queryKeys.agentCredentials(workspaceSlug, agentId),
    queryFn: ({ signal }) => listAgentCredentials(workspaceSlug, agentId, undefined, { signal }),
    enabled: workspaceSlug.length > 0 && agentId.length > 0,
  });
}

export function useCreateAgentCredential(workspaceSlug: string, agentId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateAgentCredentialPayload) =>
      createAgentCredential(workspaceSlug, agentId, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(queryKeys.agentCredential(workspaceSlug, created.credential.id), created.credential);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.agentCredentials(workspaceSlug, agentId),
      });
    },
  });
}

export function useAgentCredential(workspaceSlug: string, credentialId: string) {
  return useQuery({
    queryKey: queryKeys.agentCredential(workspaceSlug, credentialId),
    queryFn: ({ signal }) => getAgentCredential(workspaceSlug, credentialId, { signal }),
    enabled: workspaceSlug.length > 0 && credentialId.length > 0,
  });
}

export function useUpdateAgentCredential(workspaceSlug: string, credentialId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateAgentCredentialPayload) =>
      updateAgentCredential(workspaceSlug, credentialId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.agentCredential(workspaceSlug, credentialId), updated);
    },
  });
}

export function useDeleteAgentCredential(workspaceSlug: string, agentId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (credentialId: string) => deleteAgentCredential(workspaceSlug, credentialId),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.agentCredentials(workspaceSlug, agentId),
      });
    },
  });
}

export function useScopedProjects(workspaceSlug: string) {
  return useQuery({
    queryKey: queryKeys.projects(workspaceSlug),
    queryFn: ({ signal }) => listProjects(workspaceSlug, undefined, { signal }),
    enabled: workspaceSlug.length > 0,
  });
}

export type AgentFormDraft = {
  key: string;
  display_name: string;
  status: AgentSummary['status'];
};

export type CredentialFormDraft = {
  label: string;
  project_id: string;
  scopes: string[];
  expires_at: string;
  status: CredentialStatus;
};

export type AgentListData = ApiListData<AgentSummary>;
export type AgentCredentialListData = ApiListData<AgentCredentialSummary>;
