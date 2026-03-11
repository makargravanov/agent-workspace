import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import { createNote, getNote, listNotes } from '../api/notes';
import type { CreateNotePayload, PaginationParams } from '../api/types';

export function useNotes(workspaceSlug: string, projectSlug: string, pagination?: PaginationParams) {
  return useQuery({
    queryKey: queryKeys.notes(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listNotes(workspaceSlug, projectSlug, pagination, { signal }),
  });
}

export function useNote(workspaceSlug: string, projectSlug: string, noteId: string) {
  return useQuery({
    queryKey: queryKeys.note(workspaceSlug, projectSlug, noteId),
    queryFn: ({ signal }) => getNote(workspaceSlug, projectSlug, noteId, { signal }),
  });
}

export function useCreateNote(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: CreateNotePayload) =>
      createNote(workspaceSlug, projectSlug, payload),
    onSuccess: (created) => {
      queryClient.setQueryData(
        queryKeys.note(workspaceSlug, projectSlug, created.id),
        created,
      );
      void queryClient.invalidateQueries({
        queryKey: queryKeys.notes(workspaceSlug, projectSlug),
      });
    },
  });
}
