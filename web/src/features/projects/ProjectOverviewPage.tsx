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
  const cancelledTasks = tasks.filter((task) => task.status === 'cancelled');

  return (
    <section className="overviewPage">
      <div className="metricGrid">
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
          <span className="statValue">{cancelledTasks.length}</span>
          <span className="statLabel">Отменено</span>
        </article>
        <article className="statCard">
          <span className="statValue">{notes.length}</span>
          <span className="statLabel">Заметок</span>
        </article>
      </div>

      <div className="overviewGrid">
        <section className="workPanel">
          <div className="panelHeader">
            <h2>Последние задачи</h2>
            <Link
              className="secondaryButton"
              to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/tasks`}
            >
              Открыть задачи
            </Link>
          </div>

          <div className="compactList">
            {tasks.slice(0, 5).map((task) => (
              <article key={task.id} className="compactRow">
                <div>
                  <strong>{task.title}</strong>
                  {task.description_md ? <span>{task.description_md}</span> : null}
                </div>
                <span className={`statusPill status-${task.status}`}>{statusLabel(task.status)}</span>
              </article>
            ))}
            {tasks.length === 0 ? (
              <div className="emptyPanel">Задач нет</div>
            ) : null}
          </div>
        </section>

        <section className="workPanel">
          <div className="panelHeader">
            <h2>Последние заметки</h2>
            <Link
              className="secondaryButton"
              to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/notes`}
            >
              Открыть заметки
            </Link>
          </div>

          <div className="compactList">
            {notes.slice(0, 5).map((note) => (
              <article key={note.id} className="compactRow">
                <div>
                  <strong>{note.title ?? 'Без названия'}</strong>
                  <span>{note.body_md}</span>
                </div>
                <span className="statusPill">{noteKindLabel(note.kind)}</span>
              </article>
            ))}
            {notes.length === 0 ? (
              <div className="emptyPanel">Заметок нет</div>
            ) : null}
          </div>
        </section>
      </div>
    </section>
  );
}
