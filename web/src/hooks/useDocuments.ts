import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { createDocument, deleteDocument, getDocument, listDocuments, updateDocument } from '../api/documents';
import { queryKeys } from '../api/query-keys';
import type {
  ApiListData,
  CreateDocumentPayload,
  DocumentDetail,
  PaginationParams,
  UpdateDocumentPayload,
} from '../api/types';

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
