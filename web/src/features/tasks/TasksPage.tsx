import { Flag, Kanban, Plus, Search, Table2, Trash2 } from 'lucide-react';
import type { DragEvent, FormEvent, ReactNode } from 'react';
import { useMemo, useState } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import type { TaskDetail, TaskPriority, TaskStatus } from '../../api/types';
import {
  useCreateTask,
  useDeleteTask,
  useMoveTaskStatus,
  useTasks,
  useTasksLongPolling,
  useUpdateTaskStatus,
} from '../../hooks/useTasks';
import { getErrorMessage } from '../../shared/lib/errors';
import { formatDateTime, priorityLabel, statusLabel } from '../../shared/lib/text';
import { useFieldState } from '../../shared/ui/useFieldState';

const TASK_PRIORITIES: TaskPriority[] = ['low', 'normal', 'high', 'critical'];
const TASK_STATUSES: TaskStatus[] = ['todo', 'in_progress', 'done', 'cancelled'];
const ACTIVE_STATUSES = ['all', ...TASK_STATUSES] as const;
type StatusFilter = (typeof ACTIVE_STATUSES)[number];
type ViewMode = 'board' | 'list';

const COLUMNS: Array<{ status: TaskStatus; title: string }> = [
  { status: 'todo', title: 'К выполнению' },
  { status: 'in_progress', title: 'В работе' },
  { status: 'done', title: 'Готово' },
  { status: 'cancelled', title: 'Отменено' },
];

export function TasksPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const tasksQuery = useTasks(workspaceSlug, projectSlug);
  useTasksLongPolling(workspaceSlug, projectSlug, true);
  const createTaskMutation = useCreateTask(workspaceSlug, projectSlug);
  const deleteTaskMutation = useDeleteTask(workspaceSlug, projectSlug);
  const title = useFieldState('');
  const description = useFieldState('');
  const priority = useFieldState<TaskPriority>('normal');
  const [draggedTaskId, setDraggedTaskId] = useState<string | null>(null);

  const view = (searchParams.get('view') === 'list' ? 'list' : 'board') satisfies ViewMode;
  const search = searchParams.get('q') ?? '';
  const status = normalizeStatusFilter(searchParams.get('status'));
  const priorityFilter = normalizePriorityFilter(searchParams.get('priority'));
  const tasks = useMemo(() => tasksQuery.data?.items ?? [], [tasksQuery.data?.items]);

  const filteredTasks = useMemo(() => {
    const normalizedSearch = search.trim().toLowerCase();

    return tasks.filter((task) => {
      const matchesSearch =
        normalizedSearch.length === 0 ||
        task.title.toLowerCase().includes(normalizedSearch) ||
        (task.description_md?.toLowerCase().includes(normalizedSearch) ?? false);
      const matchesStatus = status === 'all' || task.status === status;
      const matchesPriority = priorityFilter === 'all' || task.priority === priorityFilter;

      return matchesSearch && matchesStatus && matchesPriority;
    });
  }, [priorityFilter, search, status, tasks]);

  function setParam(key: string, value: string) {
    const next = new URLSearchParams(searchParams);
    if (value === '' || value === 'all' || (key === 'view' && value === 'board')) {
      next.delete(key);
    } else {
      next.set(key, value);
    }
    setSearchParams(next, { replace: true });
  }

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

  function handleDeleteTask(task: TaskDetail) {
    if (!window.confirm(`Удалить задачу «${task.title}»? Это действие необратимо.`)) {
      return;
    }

    deleteTaskMutation.mutate(task.id);
  }

  return (
    <section className="trackerPage">
      <div className="trackerToolbar">
        <div className="searchField">
          <Search size={16} />
          <input
            value={search}
            onChange={(event) => setParam('q', event.target.value)}
            placeholder="Поиск задач"
          />
        </div>
        <select value={status} onChange={(event) => setParam('status', event.target.value)}>
          {ACTIVE_STATUSES.map((item) => (
            <option key={item} value={item}>
              {item === 'all' ? 'Все статусы' : statusLabel(item)}
            </option>
          ))}
        </select>
        <select value={priorityFilter} onChange={(event) => setParam('priority', event.target.value)}>
          <option value="all">Все приоритеты</option>
          {TASK_PRIORITIES.map((item) => (
            <option key={item} value={item}>
              {priorityLabel(item)}
            </option>
          ))}
        </select>
        <div className="viewSwitch" aria-label="Вид задач">
          <button
            type="button"
            className={view === 'board' ? 'isActive' : ''}
            onClick={() => setParam('view', 'board')}
            title="Доска"
          >
            <Kanban size={16} />
            <span>Доска</span>
          </button>
          <button
            type="button"
            className={view === 'list' ? 'isActive' : ''}
            onClick={() => setParam('view', 'list')}
            title="Список"
          >
            <Table2 size={16} />
            <span>Список</span>
          </button>
        </div>
      </div>

      {tasksQuery.error ? <p className="errorText">{getErrorMessage(tasksQuery.error)}</p> : null}
      {deleteTaskMutation.error ? <p className="errorText">{getErrorMessage(deleteTaskMutation.error)}</p> : null}

      {view === 'list' ? (
        <TaskList
          workspaceSlug={workspaceSlug}
          projectSlug={projectSlug}
          tasks={filteredTasks}
          onDeleteTask={handleDeleteTask}
          deletePending={deleteTaskMutation.isPending}
        />
      ) : (
        <div className="kanbanBoard">
          {COLUMNS.map((column) => (
            <KanbanColumn
              key={column.status}
              workspaceSlug={workspaceSlug}
              projectSlug={projectSlug}
              status={column.status}
              title={column.title}
              tasks={filteredTasks.filter((task) => task.status === column.status)}
              draggedTaskId={draggedTaskId}
              setDraggedTaskId={setDraggedTaskId}
              onDeleteTask={handleDeleteTask}
              deletePending={deleteTaskMutation.isPending}
            >
              {column.status === 'todo' ? (
                <QuickCreateTask
                  title={title}
                  description={description}
                  priority={priority}
                  isPending={createTaskMutation.isPending}
                  error={createTaskMutation.error}
                  onSubmit={handleSubmit}
                />
              ) : null}
            </KanbanColumn>
          ))}
        </div>
      )}
    </section>
  );
}

function KanbanColumn({
  workspaceSlug,
  projectSlug,
  status,
  title,
  tasks,
  draggedTaskId,
  setDraggedTaskId,
  onDeleteTask,
  deletePending,
  children,
}: {
  workspaceSlug: string;
  projectSlug: string;
  status: TaskStatus;
  title: string;
  tasks: TaskDetail[];
  draggedTaskId: string | null;
  setDraggedTaskId: (taskId: string | null) => void;
  onDeleteTask: (task: TaskDetail) => void;
  deletePending: boolean;
  children?: ReactNode;
}) {
  const moveStatusMutation = useMoveTaskStatus(workspaceSlug, projectSlug);
  const [isOver, setIsOver] = useState(false);

  function handleDrop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    const taskId = event.dataTransfer.getData('text/task-id') || draggedTaskId;
    const sourceStatus = event.dataTransfer.getData('text/task-status') as TaskStatus;
    setIsOver(false);
    setDraggedTaskId(null);

    if (!taskId || sourceStatus === status) {
      return;
    }

    moveStatusMutation.mutate({ taskId, status });
  }

  return (
    <section
      className={`kanbanColumn status-${status}${isOver ? ' isOver' : ''}`}
      onDragOver={(event) => {
        event.preventDefault();
        setIsOver(true);
      }}
      onDragLeave={() => setIsOver(false)}
      onDrop={handleDrop}
    >
      <div className="kanbanColumnHeader">
        <span>{title}</span>
        <strong>{tasks.length}</strong>
      </div>
      <div className="kanbanColumnBody">
        {children}
        {tasks.map((task) => (
          <TaskCard
            key={task.id}
            workspaceSlug={workspaceSlug}
            projectSlug={projectSlug}
            task={task}
            onDragStart={() => setDraggedTaskId(task.id)}
            onDragEnd={() => setDraggedTaskId(null)}
            onDelete={() => onDeleteTask(task)}
            deletePending={deletePending}
          />
        ))}
        {tasks.length === 0 && !children ? <div className="emptyLane">Нет задач</div> : null}
      </div>
      {moveStatusMutation.error ? (
        <p className="errorText">{getErrorMessage(moveStatusMutation.error)}</p>
      ) : null}
    </section>
  );
}

function QuickCreateTask({
  title,
  description,
  priority,
  isPending,
  error,
  onSubmit,
}: {
  title: ReturnType<typeof useFieldState<string>>;
  description: ReturnType<typeof useFieldState<string>>;
  priority: ReturnType<typeof useFieldState<TaskPriority>>;
  isPending: boolean;
  error: Error | null;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <form className="quickTaskForm" onSubmit={onSubmit}>
      <input
        value={title.value}
        onChange={(event) => title.setValue(event.target.value)}
        placeholder="Новая задача"
        required
      />
      <textarea
        value={description.value}
        onChange={(event) => description.setValue(event.target.value)}
        rows={2}
        placeholder="Описание"
      />
      <div className="quickTaskFooter">
        <label className="quickTaskPriorityField">
          <span>
            <Flag size={14} />
            Приоритет
          </span>
          <select
            value={priority.value}
            onChange={(event) => priority.setValue(event.target.value as TaskPriority)}
            aria-label="Приоритет задачи"
          >
            {TASK_PRIORITIES.map((item) => (
              <option key={item} value={item}>
                {priorityLabel(item)}
              </option>
            ))}
          </select>
        </label>
        <button type="submit" className="primaryButton compactButton" disabled={isPending}>
          <Plus size={15} />
          <span>{isPending ? '...' : 'Создать'}</span>
        </button>
      </div>
      {error ? <p className="errorText">{getErrorMessage(error)}</p> : null}
    </form>
  );
}

function TaskCard({
  workspaceSlug,
  projectSlug,
  task,
  onDragStart,
  onDragEnd,
  onDelete,
  deletePending,
}: {
  workspaceSlug: string;
  projectSlug: string;
  task: TaskDetail;
  onDragStart?: () => void;
  onDragEnd?: () => void;
  onDelete: () => void;
  deletePending: boolean;
}) {
  const updateStatusMutation = useUpdateTaskStatus(workspaceSlug, projectSlug, task.id);

  return (
    <article
      className="taskCard"
      draggable
      onDragStart={(event) => {
        event.dataTransfer.setData('text/task-id', task.id);
        event.dataTransfer.setData('text/task-status', task.status);
        onDragStart?.();
      }}
      onDragEnd={onDragEnd}
    >
      <div className="taskCardHeader">
        <div className="taskCardHeading">
          <span
            className={`priorityDot priority-${task.priority}`}
            title={priorityLabel(task.priority)}
          />
          <strong>{task.title}</strong>
        </div>
        <button
          type="button"
          className="iconButton dangerIconButton taskCardDelete"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onDelete();
          }}
          disabled={deletePending}
          title="Удалить задачу"
          aria-label={`Удалить задачу ${task.title}`}
        >
          <Trash2 size={14} />
        </button>
      </div>
      {task.description_md ? <p>{task.description_md}</p> : null}
      <div className="taskCardMeta">
        <span className={`statusPill status-${task.status}`}>{statusLabel(task.status)}</span>
        {task.blocked ? <span className="blockedPill">Блок</span> : null}
        <span>{formatDateTime(task.updated_at)}</span>
      </div>
      <select
        className="taskStatusSelect"
        value={task.status}
        onChange={(event) =>
          updateStatusMutation.mutate({ status: event.target.value as TaskStatus })
        }
        disabled={updateStatusMutation.isPending}
        aria-label="Статус задачи"
      >
        {TASK_STATUSES.map((item) => (
          <option key={item} value={item}>
            {statusLabel(item)}
          </option>
        ))}
      </select>
      {updateStatusMutation.error ? (
        <p className="errorText">{getErrorMessage(updateStatusMutation.error)}</p>
      ) : null}
    </article>
  );
}

function TaskList({
  workspaceSlug,
  projectSlug,
  tasks,
  onDeleteTask,
  deletePending,
}: {
  workspaceSlug: string;
  projectSlug: string;
  tasks: TaskDetail[];
  onDeleteTask: (task: TaskDetail) => void;
  deletePending: boolean;
}) {
  if (tasks.length === 0) {
    return <div className="emptyPanel">Нет задач</div>;
  }

  return (
    <div className="tablePanel">
      <table className="taskTable">
        <thead>
          <tr>
            <th>Задача</th>
            <th>Статус</th>
            <th>Приоритет</th>
            <th>Обновлена</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {tasks.map((task) => (
            <TaskRow
              key={task.id}
              workspaceSlug={workspaceSlug}
              projectSlug={projectSlug}
              task={task}
              onDelete={() => onDeleteTask(task)}
              deletePending={deletePending}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function TaskRow({
  workspaceSlug,
  projectSlug,
  task,
  onDelete,
  deletePending,
}: {
  workspaceSlug: string;
  projectSlug: string;
  task: TaskDetail;
  onDelete: () => void;
  deletePending: boolean;
}) {
  const updateStatusMutation = useUpdateTaskStatus(workspaceSlug, projectSlug, task.id);

  return (
    <tr>
      <td>
        <strong>{task.title}</strong>
        {task.description_md ? <span>{task.description_md}</span> : null}
      </td>
      <td>
        <span className={`statusPill status-${task.status}`}>{statusLabel(task.status)}</span>
      </td>
      <td>{priorityLabel(task.priority)}</td>
      <td>{formatDateTime(task.updated_at)}</td>
      <td>
        <div className="tableActionsCell">
          <select
            value={task.status}
            onChange={(event) =>
              updateStatusMutation.mutate({ status: event.target.value as TaskStatus })
            }
            disabled={updateStatusMutation.isPending}
            aria-label="Статус задачи"
          >
            {TASK_STATUSES.map((item) => (
              <option key={item} value={item}>
                {statusLabel(item)}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="iconButton dangerIconButton"
            onClick={onDelete}
            disabled={deletePending}
            title="Удалить задачу"
            aria-label={`Удалить задачу ${task.title}`}
          >
            <Trash2 size={14} />
          </button>
        </div>
      </td>
    </tr>
  );
}

function normalizeStatusFilter(value: string | null): StatusFilter {
  return ACTIVE_STATUSES.includes(value as StatusFilter) ? (value as StatusFilter) : 'all';
}

function normalizePriorityFilter(value: string | null): TaskPriority | 'all' {
  return TASK_PRIORITIES.includes(value as TaskPriority) ? (value as TaskPriority) : 'all';
}
