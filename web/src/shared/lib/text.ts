import type { TaskPriority, TaskStatus } from '../../api/types';

export function slugify(value: string): string {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

export function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat('ru-RU', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value));
}

export function statusLabel(status: TaskStatus): string {
  switch (status) {
    case 'todo':
      return 'К выполнению';
    case 'in_progress':
      return 'В работе';
    case 'done':
      return 'Готово';
    case 'cancelled':
      return 'Отменено';
  }
}

export function priorityLabel(priority: TaskPriority): string {
  switch (priority) {
    case 'low':
      return 'Низкий';
    case 'normal':
      return 'Обычный';
    case 'high':
      return 'Высокий';
    case 'critical':
      return 'Критический';
  }
}
