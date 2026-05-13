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
  'failed to list agents': 'Не удалось загрузить агентов.',
  'failed to create agent': 'Не удалось создать агента.',
  'failed to update agent': 'Не удалось обновить агента.',
  'failed to delete agent': 'Не удалось удалить агента.',
  'failed to resolve agent': 'Не удалось определить агента.',
  'failed to list agent credentials': 'Не удалось загрузить credentials.',
  'failed to create agent credential': 'Не удалось выпустить credential.',
  'failed to update agent credential': 'Не удалось обновить credential.',
  'failed to delete agent credential': 'Не удалось удалить credential.',
  'failed to resolve agent credential': 'Не удалось определить credential.',
  'failed to list tasks': 'Не удалось загрузить задачи.',
  'failed to create task': 'Не удалось создать задачу.',
  'failed to update task': 'Не удалось обновить задачу.',
  'failed to delete task': 'Не удалось удалить задачу.',
  'failed to list notes': 'Не удалось загрузить заметки.',
  'failed to create note': 'Не удалось создать заметку.',
  'failed to delete note': 'Не удалось удалить заметку.',
  'failed to list documents': 'Не удалось загрузить документы.',
  'failed to create document': 'Не удалось создать документ.',
  'failed to update document': 'Не удалось обновить документ.',
  'failed to delete document': 'Не удалось удалить документ.',
  'failed to list integration connections': 'Не удалось загрузить подключения.',
  'failed to create integration connection': 'Не удалось создать подключение.',
  'failed to update integration connection': 'Не удалось обновить подключение.',
  'failed to delete integration connection': 'Не удалось удалить подключение.',
  'project_id is required when scope_kind = project':
    'Для области проекта нужно указать проект.',
  'project_id is only allowed when scope_kind = project':
    'project_id допустим только для области проекта.',
  'project not found in this workspace': 'Проект не найден в этом рабочем пространстве.',
  'connection not found': 'Подключение не найдено.',
  'integration_connection_not_found': 'Подключение не найдено.',
  'provider must be github': 'Провайдер должен быть GitHub.',
  'scope_kind must be one of: workspace, project':
    'Тип области должен быть workspace или project.',
  'status must be one of: active, disabled, error':
    'Статус должен быть active, disabled или error.',
  'Config JSON is invalid.': 'JSON конфигурации некорректен.',
  'JSON конфигурации некорректен.': 'JSON конфигурации некорректен.',
  'document version is stale; reload before updating':
    'Документ был изменён на сервере. Сначала обновите его и повторите сохранение.',
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
