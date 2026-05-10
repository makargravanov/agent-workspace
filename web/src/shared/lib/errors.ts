import { ApiError } from '../../api/client';

const MESSAGE_MAP: Record<string, string> = {
  'failed to list workspaces': 'Не удалось загрузить рабочие пространства.',
  'failed to create workspace': 'Не удалось создать рабочее пространство.',
  'failed to delete workspace': 'Не удалось удалить рабочее пространство.',
  'failed to resolve workspace': 'Не удалось определить рабочее пространство.',
  'failed to resolve current workspace member':
    'Не удалось определить текущего участника рабочего пространства.',
  'failed to fetch workspace': 'Не удалось загрузить рабочее пространство.',
  'failed to list projects': 'Не удалось загрузить проекты.',
  'failed to create project': 'Не удалось создать проект.',
  'failed to delete project': 'Не удалось удалить проект.',
  'failed to resolve project': 'Не удалось определить проект.',
  'failed to list tasks': 'Не удалось загрузить задачи.',
  'failed to create task': 'Не удалось создать задачу.',
  'failed to update task': 'Не удалось обновить задачу.',
  'failed to delete task': 'Не удалось удалить задачу.',
  'failed to list notes': 'Не удалось загрузить заметки.',
  'failed to create note': 'Не удалось создать заметку.',
  'failed to delete note': 'Не удалось удалить заметку.',
  'authentication is required': 'Требуется авторизация.',
};

export function getErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return MESSAGE_MAP[error.message] ?? error.message;
  }

  if (error instanceof Error) {
    return MESSAGE_MAP[error.message] ?? error.message;
  }

  return 'Не удалось выполнить запрос.';
}
