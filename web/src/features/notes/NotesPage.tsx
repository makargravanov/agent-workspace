import { Plus, Trash2 } from 'lucide-react';
import type { FormEvent } from 'react';
import { useParams } from 'react-router-dom';
import type { NoteKind } from '../../api/types';
import { useCreateNote, useDeleteNote, useNotes } from '../../hooks/useNotes';
import { getErrorMessage } from '../../shared/lib/errors';
import { formatDateTime, noteKindLabel } from '../../shared/lib/text';
import { useFieldState } from '../../shared/ui/useFieldState';

const NOTE_KINDS: NoteKind[] = ['context', 'worklog', 'decision', 'result'];

export function NotesPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const notesQuery = useNotes(workspaceSlug, projectSlug);
  const createNoteMutation = useCreateNote(workspaceSlug, projectSlug);
  const deleteNoteMutation = useDeleteNote(workspaceSlug, projectSlug);
  const kind = useFieldState<NoteKind>('context');
  const title = useFieldState('');
  const body = useFieldState('');

  const notes = notesQuery.data?.items ?? [];

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createNoteMutation.mutate(
      {
        kind: kind.value,
        title: title.value.trim() || null,
        body_md: body.value.trim(),
      },
      {
        onSuccess: () => {
          kind.setValue('context');
          title.setValue('');
          body.setValue('');
        },
      },
    );
  }

  function handleDeleteNote(noteId: string, noteTitle: string) {
    if (!window.confirm(`Удалить заметку «${noteTitle}»? Это действие необратимо.`)) {
      return;
    }

    deleteNoteMutation.mutate(noteId);
  }

  return (
    <section className="notesPage">
      <section className="composePanel">
        <div className="compactTitle">
          <Plus size={16} />
          <h2>Создать заметку</h2>
        </div>

        <form className="formGrid formGridWide" onSubmit={handleSubmit}>
          <label className="field">
            <span>Тип</span>
            <select
              value={kind.value}
              onChange={(event) => kind.setValue(event.target.value as NoteKind)}
            >
              {NOTE_KINDS.map((item) => (
                <option key={item} value={item}>
                  {noteKindLabel(item)}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>Заголовок</span>
            <input
              value={title.value}
              onChange={(event) => title.setValue(event.target.value)}
              placeholder="Решение или контекст"
            />
          </label>
          <label className="field fieldSpan2">
            <span>Текст заметки</span>
            <textarea
              value={body.value}
              onChange={(event) => body.setValue(event.target.value)}
              rows={6}
              placeholder="Сохраните текст заметки."
              required
            />
          </label>
          <div className="formActions">
            <button
              type="submit"
              className="primaryButton compactButton"
              disabled={createNoteMutation.isPending}
            >
              {createNoteMutation.isPending ? 'Сохранение...' : 'Создать'}
            </button>
          </div>
        </form>

        {createNoteMutation.error ? (
          <p className="errorText">{getErrorMessage(createNoteMutation.error)}</p>
        ) : null}
        {deleteNoteMutation.error ? (
          <p className="errorText">{getErrorMessage(deleteNoteMutation.error)}</p>
        ) : null}
      </section>

      <section className="workPanel">
        <div className="compactList">
          {notes.map((note) => (
            <article key={note.id} className="noteRow">
              <div className="noteRowHeader">
                <div className="noteRowHeading">
                  <strong>{note.title ?? 'Без названия'}</strong>
                  <span className="statusPill">{noteKindLabel(note.kind)}</span>
                </div>
                <button
                  type="button"
                  className="iconButton dangerIconButton noteDeleteButton"
                  onClick={() => handleDeleteNote(note.id, note.title ?? 'Без названия')}
                  disabled={deleteNoteMutation.isPending}
                  title="Удалить заметку"
                  aria-label={`Удалить заметку ${note.title ?? 'Без названия'}`}
                >
                  <Trash2 size={14} />
                </button>
              </div>
              <p>{note.body_md}</p>
              <div className="noteMeta">
                <span className="mutedText">{note.author_type}</span>
                <span className="mutedText">{formatDateTime(note.updated_at)}</span>
              </div>
            </article>
          ))}
          {notes.length === 0 ? (
            <div className="emptyPanel">Заметок нет</div>
          ) : null}
        </div>
      </section>
    </section>
  );
}
