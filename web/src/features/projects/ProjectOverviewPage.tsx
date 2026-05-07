import { Link, useParams } from 'react-router-dom';
import { useNotes } from '../../hooks/useNotes';
import { useTasks } from '../../hooks/useTasks';
import { noteKindLabel, statusLabel } from '../../shared/lib/text';

export function ProjectOverviewPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const tasksQuery = useTasks(workspaceSlug, projectSlug);
  const notesQuery = useNotes(workspaceSlug, projectSlug);
  const tasks = tasksQuery.data?.items ?? [];
  const notes = notesQuery.data?.items ?? [];
  const activeTasks = tasks.filter((task) => task.status === 'todo' || task.status === 'in_progress');
  const completedTasks = tasks.filter((task) => task.status === 'done');

  return (
    <section className="pageStack">
      <div className="statsGrid">
        <article className="statCard">
          <span className="statValue">{tasks.length}</span>
          <span className="statLabel">Всего задач</span>
        </article>
        <article className="statCard">
          <span className="statValue">{activeTasks.length}</span>
          <span className="statLabel">Активных задач</span>
        </article>
        <article className="statCard">
          <span className="statValue">{completedTasks.length}</span>
          <span className="statLabel">Готово</span>
        </article>
        <article className="statCard">
          <span className="statValue">{notes.length}</span>
          <span className="statLabel">Заметок</span>
        </article>
      </div>

      <div className="overviewGrid">
        <section className="panel">
          <div className="panelHeader">
            <div>
              <h2>Последние задачи</h2>
            </div>
            <Link
              className="secondaryButton"
              to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/tasks`}
            >
              Открыть задачи
            </Link>
          </div>

          <div className="entityList">
            {tasks.slice(0, 5).map((task) => (
              <article key={task.id} className="entityCard">
                <div className="summaryRow">
                  <strong>{task.title}</strong>
                  <span className="statusBadge">{statusLabel(task.status)}</span>
                </div>
                {task.description_md ? <p>{task.description_md}</p> : null}
              </article>
            ))}
            {tasks.length === 0 ? (
              <div className="emptyPanel">
                <h3>Задач пока нет</h3>
              </div>
            ) : null}
          </div>
        </section>

        <section className="panel">
          <div className="panelHeader">
            <div>
              <h2>Последние заметки</h2>
            </div>
            <Link
              className="secondaryButton"
              to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/notes`}
            >
              Открыть заметки
            </Link>
          </div>

          <div className="entityList">
            {notes.slice(0, 5).map((note) => (
              <article key={note.id} className="entityCard">
                <div className="summaryRow">
                  <strong>{note.title ?? 'Без названия'}</strong>
                  <span className="statusBadge">{noteKindLabel(note.kind)}</span>
                </div>
                <p>{note.body_md}</p>
              </article>
            ))}
            {notes.length === 0 ? (
              <div className="emptyPanel">
                <h3>Заметок пока нет</h3>
              </div>
            ) : null}
          </div>
        </section>
      </div>
    </section>
  );
}
