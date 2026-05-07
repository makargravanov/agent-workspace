import type { FormEvent } from 'react';
import { useParams } from 'react-router-dom';
import type { NoteKind } from '../../api/types';
import { useCreateNote, useNotes } from '../../hooks/useNotes';
import { getErrorMessage } from '../../shared/lib/errors';
import { formatDateTime } from '../../shared/lib/text';
import { useFieldState } from '../../shared/ui/useFieldState';

const NOTE_KINDS: NoteKind[] = ['context', 'worklog', 'decision', 'result'];

export function NotesPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const notesQuery = useNotes(workspaceSlug, projectSlug);
  const createNoteMutation = useCreateNote(workspaceSlug, projectSlug);
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

  return (
    <section className="pageStack">
      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Создать заметку</h2>
            <p className="mutedText">Заметки работают с реальным knowledge-base endpoint.</p>
          </div>
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
                  {item}
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
              placeholder="Сохрани контекст в markdown."
              required
            />
          </label>
          <div className="formActions">
            <button
              type="submit"
              className="primaryButton"
              disabled={createNoteMutation.isPending}
            >
              {createNoteMutation.isPending ? 'Сохранение...' : 'Создать заметку'}
            </button>
          </div>
        </form>

        {createNoteMutation.error ? (
          <p className="errorText">{getErrorMessage(createNoteMutation.error)}</p>
        ) : null}
      </section>

      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Список заметок</h2>
            <p className="mutedText">Заметки выводятся отдельно от overview-экрана.</p>
          </div>
        </div>

        <div className="entityList">
          {notes.map((note) => (
            <article key={note.id} className="entityCard">
              <div className="summaryRow">
                <strong>{note.title ?? 'Без названия'}</strong>
                <span className="statusBadge">{note.kind}</span>
              </div>
              <p>{note.body_md}</p>
              <div className="summaryRow">
                <span className="mutedText">{note.author_type}</span>
                <span className="mutedText">{formatDateTime(note.updated_at)}</span>
              </div>
            </article>
          ))}
          {notes.length === 0 ? (
            <div className="emptyPanel">
              <h3>Заметок пока нет</h3>
              <p>Создай первую заметку через форму выше.</p>
            </div>
          ) : null}
        </div>
      </section>
    </section>
  );
}
