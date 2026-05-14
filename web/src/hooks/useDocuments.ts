import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import {
  createDocument,
  deleteDocument,
  getDocument,
  listDocuments,
  moveDocument,
  repairDocumentCycles,
  updateDocument,
  waitForDocumentChanges,
} from '../api/documents';
import { queryKeys } from '../api/query-keys';
import type {
  ApiListData,
  CreateDocumentPayload,
  DocumentDetail,
  PaginationParams,
  UpdateDocumentPayload,
} from '../api/types';

const POLL_RETRY_DELAY_MS = 2_000;

function isAbortError(error: unknown) {
  return error instanceof DOMException && error.name === 'AbortError';
}

function delay(ms: number, signal: AbortSignal) {
  return new Promise<void>((resolve, reject) => {
    if (signal.aborted) {
      reject(new DOMException('Aborted', 'AbortError'));
      return;
    }

    const timeoutId = window.setTimeout(resolve, ms);
    signal.addEventListener(
      'abort',
      () => {
        window.clearTimeout(timeoutId);
        reject(new DOMException('Aborted', 'AbortError'));
      },
      { once: true },
    );
  });
}

export function useDocumentsLongPolling(
  workspaceSlug: string,
  projectSlug: string,
  enabled: boolean,
  activeDocumentId?: string,
) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!enabled || workspaceSlug.length === 0 || projectSlug.length === 0) {
      return;
    }

    const controller = new AbortController();

    async function poll() {
      let cursor: string | undefined;

      while (!controller.signal.aborted) {
        try {
          const result = await waitForDocumentChanges(workspaceSlug, projectSlug, cursor, {
            signal: controller.signal,
          });
          cursor = result.cursor;

          if (result.changed) {
            await queryClient.invalidateQueries({
              queryKey: queryKeys.documents(workspaceSlug, projectSlug),
            });
            if (activeDocumentId && activeDocumentId.length > 0) {
              await queryClient.invalidateQueries({
                queryKey: queryKeys.document(workspaceSlug, projectSlug, activeDocumentId),
              });
            }
          }
        } catch (error) {
          if (isAbortError(error)) {
            return;
          }
          await delay(POLL_RETRY_DELAY_MS, controller.signal).catch(() => undefined);
        }
      }
    }

    void poll();

    return () => {
      controller.abort();
    };
  }, [activeDocumentId, enabled, projectSlug, queryClient, workspaceSlug]);
}

export function useDocuments(
  workspaceSlug: string,
  projectSlug: string,
  pagination?: PaginationParams,
) {
  return useQuery({
    queryKey: queryKeys.documents(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listDocuments(workspaceSlug, projectSlug, pagination, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
  });
}

export function useDocument(workspaceSlug: string, projectSlug: string, documentId: string) {
  return useQuery({
    queryKey: queryKeys.document(workspaceSlug, projectSlug, documentId),
    queryFn: ({ signal }) => getDocument(workspaceSlug, projectSlug, documentId, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0 && documentId.length > 0,
  });
}

export function useCreateDocument(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateDocumentPayload) =>
      createDocument(workspaceSlug, projectSlug, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(queryKeys.document(workspaceSlug, projectSlug, created.id), created);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.documents(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useUpdateDocument(workspaceSlug: string, projectSlug: string, documentId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateDocumentPayload) =>
      updateDocument(workspaceSlug, projectSlug, documentId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.document(workspaceSlug, projectSlug, documentId), updated);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.documents(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useReparentDocument(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      documentId,
      payload,
    }: {
      documentId: string;
      payload: UpdateDocumentPayload;
    }) => updateDocument(workspaceSlug, projectSlug, documentId, payload),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.document(workspaceSlug, projectSlug, updated.id), updated);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.documents(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useDeleteDocument(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (documentId: string) => deleteDocument(workspaceSlug, projectSlug, documentId),
    onMutate: async (documentId) => {
      const documentsKey = queryKeys.documents(workspaceSlug, projectSlug);
      const documentKey = queryKeys.document(workspaceSlug, projectSlug, documentId);

      await queryClient.cancelQueries({ queryKey: documentsKey });
      await queryClient.cancelQueries({ queryKey: documentKey });

      const previousDocuments = queryClient.getQueryData<ApiListData<DocumentDetail>>(documentsKey);
      const previousDocument = queryClient.getQueryData<DocumentDetail>(documentKey);

      queryClient.setQueryData<ApiListData<DocumentDetail>>(documentsKey, (current) =>
        current
          ? {
              ...current,
              items: current.items.filter((document) => document.id !== documentId),
            }
          : current,
      );
      queryClient.removeQueries({ queryKey: documentKey });

      return { previousDocuments, previousDocument };
    },
    onError: (_error, _documentId, context) => {
      if (context?.previousDocuments) {
        queryClient.setQueryData(
          queryKeys.documents(workspaceSlug, projectSlug),
          context.previousDocuments,
        );
      }
      if (context?.previousDocument) {
        queryClient.setQueryData(
          queryKeys.document(workspaceSlug, projectSlug, context.previousDocument.id),
          context.previousDocument,
        );
      }
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.documents(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useRepairDocumentCycles(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => repairDocumentCycles(workspaceSlug, projectSlug),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.documents(workspaceSlug, projectSlug),
      });
    },
  });
}

export function useMoveDocument(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      documentId,
      targetParentDocumentId,
    }: {
      documentId: string;
      targetParentDocumentId: string | null;
    }) => moveDocument(workspaceSlug, projectSlug, documentId, targetParentDocumentId),
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.document(workspaceSlug, projectSlug, updated.id), updated);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.documents(workspaceSlug, projectSlug),
      });
    },
  });
}
