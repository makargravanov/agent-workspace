import type { FormEvent } from 'react';
import { useParams } from 'react-router-dom';
import type { TaskPriority, TaskStatus } from '../../api/types';
import { useCreateTask, useTasks, useUpdateTaskStatus } from '../../hooks/useTasks';
import { getErrorMessage } from '../../shared/lib/errors';
import { formatDateTime, priorityLabel, statusLabel } from '../../shared/lib/text';
import { useFieldState } from '../../shared/ui/useFieldState';

const TASK_PRIORITIES: TaskPriority[] = ['low', 'normal', 'high', 'critical'];
const TASK_STATUSES: TaskStatus[] = ['todo', 'in_progress', 'done', 'cancelled'];

export function TasksPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const tasksQuery = useTasks(workspaceSlug, projectSlug);
  const createTaskMutation = useCreateTask(workspaceSlug, projectSlug);
  const title = useFieldState('');
  const description = useFieldState('');
  const priority = useFieldState<TaskPriority>('normal');

  const tasks = tasksQuery.data?.items ?? [];

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createTaskMutation.mutate(
      {
        title: title.value.trim(),
        description_md: description.value.trim() || null,
        priority: priority.value,
      },
      {
        onSuccess: () => {
          title.setValue('');
          description.setValue('');
          priority.setValue('normal');
        },
      },
    );
  }

  return (
    <section className="pageStack">
      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Создать задачу</h2>
          </div>
        </div>

        <form className="formGrid formGridWide" onSubmit={handleSubmit}>
          <label className="field">
            <span>Название</span>
            <input
              value={title.value}
              onChange={(event) => title.setValue(event.target.value)}
              placeholder="Собрать экран задач"
              required
            />
          </label>
          <label className="field">
            <span>Приоритет</span>
            <select
              value={priority.value}
              onChange={(event) => priority.setValue(event.target.value as TaskPriority)}
            >
              {TASK_PRIORITIES.map((item) => (
                <option key={item} value={item}>
                  {priorityLabel(item)}
                </option>
              ))}
            </select>
          </label>
          <label className="field fieldSpan2">
            <span>Описание</span>
            <textarea
              value={description.value}
              onChange={(event) => description.setValue(event.target.value)}
              rows={4}
              placeholder="Что должно быть сделано."
            />
          </label>
          <div className="formActions">
            <button
              type="submit"
              className="primaryButton"
              disabled={createTaskMutation.isPending}
            >
              {createTaskMutation.isPending ? 'Создание...' : 'Создать'}
            </button>
          </div>
        </form>

        {createTaskMutation.error ? (
          <p className="errorText">{getErrorMessage(createTaskMutation.error)}</p>
        ) : null}
      </section>

      <section className="panel">
        <div className="panelHeader">
          <div>
            <h2>Список задач</h2>
          </div>
        </div>

        <div className="entityList">
          {tasks.map((task) => (
            <TaskCard
              key={task.id}
              workspaceSlug={workspaceSlug}
              projectSlug={projectSlug}
              taskId={task.id}
              title={task.title}
              description={task.description_md}
              priority={task.priority}
              status={task.status}
              blocked={task.blocked}
              updatedAt={task.updated_at}
            />
          ))}
          {tasks.length === 0 ? (
            <div className="emptyPanel">
              <h3>Задач пока нет</h3>
            </div>
          ) : null}
        </div>
      </section>
    </section>
  );
}

type TaskCardProps = {
  workspaceSlug: string;
  projectSlug: string;
  taskId: string;
  title: string;
  description: string | null;
  priority: TaskPriority;
  status: TaskStatus;
  blocked: boolean;
  updatedAt: string;
};

function TaskCard(props: TaskCardProps) {
  const updateStatusMutation = useUpdateTaskStatus(
    props.workspaceSlug,
    props.projectSlug,
    props.taskId,
  );

  return (
    <article className="entityCard">
      <div className="summaryRow">
        <strong>{props.title}</strong>
        <span className={`statusBadge priority-${props.priority}`}>{priorityLabel(props.priority)}</span>
      </div>
      {props.description ? <p>{props.description}</p> : null}
      <div className="summaryRow">
        <label className="inlineField">
          <span>Статус</span>
          <select
            value={props.status}
            onChange={(event) =>
              updateStatusMutation.mutate({ status: event.target.value as TaskStatus })
            }
            disabled={updateStatusMutation.isPending}
          >
            {TASK_STATUSES.map((item) => (
              <option key={item} value={item}>
                {statusLabel(item)}
              </option>
            ))}
          </select>
        </label>
        <span className="mutedText">{formatDateTime(props.updatedAt)}</span>
      </div>
      {props.blocked ? <p className="warningText">Задача заблокирована зависимостями.</p> : null}
      {updateStatusMutation.error ? (
        <p className="errorText">{getErrorMessage(updateStatusMutation.error)}</p>
      ) : null}
    </article>
  );
}
