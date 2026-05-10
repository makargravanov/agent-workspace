import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { queryKeys } from '../api/query-keys';
import { createNote, deleteNote, getNote, listNotes } from '../api/notes';
import type { ApiListData, CreateNotePayload, NoteDetail, PaginationParams } from '../api/types';

export function useNotes(workspaceSlug: string, projectSlug: string, pagination?: PaginationParams) {
  return useQuery({
    queryKey: queryKeys.notes(workspaceSlug, projectSlug),
    queryFn: ({ signal }) => listNotes(workspaceSlug, projectSlug, pagination, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0,
  });
}

export function useNote(workspaceSlug: string, projectSlug: string, noteId: string) {
  return useQuery({
    queryKey: queryKeys.note(workspaceSlug, projectSlug, noteId),
    queryFn: ({ signal }) => getNote(workspaceSlug, projectSlug, noteId, { signal }),
    enabled: workspaceSlug.length > 0 && projectSlug.length > 0 && noteId.length > 0,
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

export function useDeleteNote(workspaceSlug: string, projectSlug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (noteId: string) => deleteNote(workspaceSlug, projectSlug, noteId),
    onMutate: async (noteId) => {
      const notesKey = queryKeys.notes(workspaceSlug, projectSlug);
      const noteKey = queryKeys.note(workspaceSlug, projectSlug, noteId);

      await queryClient.cancelQueries({ queryKey: notesKey });
      await queryClient.cancelQueries({ queryKey: noteKey });

      const previousNotes = queryClient.getQueryData<ApiListData<NoteDetail>>(notesKey);
      const previousNote = queryClient.getQueryData<NoteDetail>(noteKey);

      queryClient.setQueryData<ApiListData<NoteDetail>>(notesKey, (current) =>
        current
          ? {
              ...current,
              items: current.items.filter((note) => note.id !== noteId),
            }
          : current,
      );
      queryClient.removeQueries({ queryKey: noteKey });

      return { previousNotes, previousNote };
    },
    onError: (_error, _noteId, context) => {
      if (context?.previousNotes) {
        queryClient.setQueryData(queryKeys.notes(workspaceSlug, projectSlug), context.previousNotes);
      }
      if (context?.previousNote) {
        queryClient.setQueryData(
          queryKeys.note(workspaceSlug, projectSlug, context.previousNote.id),
          context.previousNote,
        );
      }
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.notes(workspaceSlug, projectSlug),
      });
    },
  });
}
