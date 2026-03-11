# API Contract: agent-workspace

## 1. Назначение

Этот документ фиксирует первичный HTTP API-контракт для ближайшей реализации.

Приоритет этого документа выше, чем у диаграмм: сначала должен быть согласован текстовый контракт, по которому можно проектировать handlers, access checks, migrations и MCP-bridge.

## 2. Границы контракта

- базовый namespace API: `/api/v1`;
- transport: HTTP + JSON, кроме загрузки и скачивания assets;
- один основной API обслуживает web-клиент, operator-контур и MCP-bridge;
- GitHub выступает только как внешняя read-only интеграция и mirror контекста;
- semantic search, embeddings и `pgvector` не входят в ближайший контракт;
- SQLite local/dev профиль считается допустимым backend-ом для локальной работы, но не целевым эталоном production-поведения.

## 3. Общие правила

- идентификаторы сущностей в payload передаются как UUID-строки;
- `workspaceSlug` и `projectSlug` используются в path для пользовательских маршрутов;
- все timestamps передаются в UTC/RFC3339;
- успешные ответы на mutation-операции по возможности возвращают обновленный resource и `audit_event_id`;
- list-эндпойнты используют cursor-based pagination там, где список может расти без верхней границы;
- server-generated `request_id` возвращается в заголовке и в error envelope;
- optimistic locking нужен прежде всего для `Document`; минимальный способ первой итерации - поле `version` в payload и проверка на update.

## 4. Аутентификация и контуры доступа

### 4.1 Human/web контур

- аутентификация через серверную сессию и защищенную cookie;
- стартовый внешний provider: GitHub OAuth;
- dev-only режим может использовать отдельный endpoint локального входа;
- `owner` - единственная роль с доступом к workspace-admin разделам.

### 4.2 Agent/MCP контур

- аутентификация через `Authorization: Bearer <agent credential>`;
- прямой `AgentCredential` является конечным средством аутентификации в MVP;
- scope-набор проверяется на каждом запросе;
- сессионная human-cookie не используется в агентном контуре.

### 4.3 System operator контур

- доступ задается через system-level allowlist аутентифицированных external subjects;
- отдельная постоянная модель `PlatformOperator` в данных пока не вводится.

## 5. Базовые форматы ответов

### 5.1 Успешный ответ с сущностью

```json
{
  "data": {
    "id": "d4f2f0b0-7b88-4e97-9c4c-8cc2f3512f25"
  },
  "meta": {
    "request_id": "req_123",
    "audit_event_id": "3c17b0dc-66da-4ebc-bff9-1cf1d7fa8b6f"
  }
}
```

### 5.2 Успешный list-ответ

```json
{
  "data": {
    "items": [],
    "next_cursor": null
  },
  "meta": {
    "request_id": "req_123"
  }
}
```

### 5.3 Ошибка

```json
{
  "error": {
    "code": "task_not_found",
    "message": "Task was not found in this project",
    "details": null,
    "request_id": "req_123"
  }
}
```

## 6. Ключевые resource-представления

### 6.1 Workspace summary

```json
{
  "id": "uuid",
  "slug": "core-platform",
  "name": "Core Platform"
}
```

### 6.2 Project summary

```json
{
  "id": "uuid",
  "workspace_id": "uuid",
  "slug": "agent-workspace",
  "name": "agent-workspace",
  "status": "active"
}
```

### 6.3 Task group summary

```json
{
  "id": "uuid",
  "project_id": "uuid",
  "kind": "initiative",
  "title": "Domain foundation",
  "status": "active",
  "priority": 100
}
```

### 6.4 Task detail

```json
{
  "id": "uuid",
  "project_id": "uuid",
  "group_id": "uuid",
  "parent_task_id": null,
  "title": "Describe initial API contract",
  "description_md": "...",
  "status": "todo",
  "priority": "high",
  "rank_key": "a0",
  "starts_at": null,
  "due_at": null,
  "assignee_type": "workspace_member",
  "assignee_id": "uuid",
  "blocked": false,
  "created_at": "2026-03-11T10:00:00Z",
  "updated_at": "2026-03-11T10:00:00Z"
}
```

### 6.5 Document detail

```json
{
  "id": "uuid",
  "project_id": "uuid",
  "parent_document_id": null,
  "slug": "api-contract",
  "title": "API Contract",
  "body_format": "markdown",
  "body_md": "# ...",
  "status": "draft",
  "version": 3,
  "created_at": "2026-03-11T10:00:00Z",
  "updated_at": "2026-03-11T11:00:00Z"
}
```

### 6.6 Note detail

```json
{
  "id": "uuid",
  "project_id": "uuid",
  "agent_session_id": null,
  "kind": "decision",
  "author_type": "workspace_member",
  "author_id": "uuid",
  "title": "SQLite profile is local-only",
  "body_md": "...",
  "created_at": "2026-03-11T10:00:00Z",
  "updated_at": "2026-03-11T10:00:00Z"
}
```

## 7. Endpoint matrix

### 7.1 System and auth

| Method | Path | Назначение | Контур | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/health` | health-check API | public | foundation |
| `GET` | `/api/v1/auth/session` | получить текущую human session | human | foundation |
| `GET` | `/api/v1/auth/github/start` | старт GitHub OAuth | public | foundation |
| `GET` | `/api/v1/auth/github/callback` | callback GitHub OAuth | public | foundation |
| `POST` | `/api/v1/auth/dev/login` | dev-only вход преднастроенным пользователем | human/dev | foundation |
| `POST` | `/api/v1/auth/logout` | завершить human session | human | foundation |

### 7.2 Workspaces and projects

| Method | Path | Назначение | Доступ | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/workspaces` | список доступных workspace | human | foundation |
| `POST` | `/api/v1/workspaces` | создать workspace | human | foundation |
| `GET` | `/api/v1/workspaces/{workspaceSlug}` | получить workspace summary | human | foundation |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects` | список проектов workspace | human | foundation |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/projects` | создать проект | `owner` | foundation |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}` | получить project summary | human | foundation |

### 7.3 Workspace admin

| Method | Path | Назначение | Доступ | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/members` | список участников | `owner` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/members/invites` | пригласить участника | `owner` | mvp |
| `PATCH` | `/api/v1/workspaces/{workspaceSlug}/members/{memberId}` | изменить роль или статус | `owner` | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/agents` | список агентных учеток | `owner` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/agents` | создать агентную учетку | `owner` | mvp |
| `PATCH` | `/api/v1/workspaces/{workspaceSlug}/agents/{agentId}` | отключить или обновить агента | `owner` | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/agent-credentials` | список credential metadata | `owner` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/agents/{agentId}/credentials` | выпустить credential | `owner` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/agent-credentials/{credentialId}/revoke` | отозвать credential | `owner` | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/integrations` | список интеграций | `owner` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/integrations/github` | подключить GitHub | `owner` | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/audit` | workspace-level audit | `owner` | mvp |

### 7.4 Task groups

| Method | Path | Назначение | Доступ | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/task-groups` | список групп задач | human, agent `task_groups:read` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/task-groups` | создать группу задач | human | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/task-groups/{groupId}` | получить группу задач | human, agent `task_groups:read` | mvp |
| `PATCH` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/task-groups/{groupId}` | обновить metadata группы | human | mvp |

### 7.5 Tasks and dependencies

| Method | Path | Назначение | Доступ | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks` | список задач с фильтрами | human, agent `tasks:read` | foundation |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks` | создать задачу | human | foundation |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}` | получить задачу | human, agent `tasks:read` | foundation |
| `PATCH` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}` | обновить задачу | human | mvp |
| `PATCH` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}/status` | ограниченно обновить статус | human, agent `tasks:write_status` | foundation |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}/dependencies` | получить блокировки и зависимости | human, agent `tasks:read` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}/dependencies` | создать зависимость | human | mvp |
| `DELETE` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}/dependencies/{dependencyId}` | удалить зависимость | human | mvp |

### 7.6 Documents, notes and assets

| Method | Path | Назначение | Доступ | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/documents` | дерево или список документов | human, agent `documents:read` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/documents` | создать документ | human | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/documents/{documentId}` | получить документ | human, agent `documents:read` | mvp |
| `PATCH` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/documents/{documentId}` | обновить документ | human | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/notes` | список заметок | human, agent `notes:read` | foundation |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/notes` | создать заметку | human, agent `notes:write` | foundation |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/notes/{noteId}` | получить заметку | human, agent `notes:read` | mvp |
| `POST` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/assets/uploads` | зарегистрировать upload asset | human | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/assets/{assetId}` | получить metadata asset | human, agent `assets:read` | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/assets/{assetId}/download` | скачать asset | human, agent `assets:read` | mvp |

### 7.7 Search and activity

| Method | Path | Назначение | Доступ | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/search` | полнотекстовый поиск по проекту | human | mvp |
| `GET` | `/api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/activity` | последние связанные изменения | human, agent `audit:read_recent` | foundation |

Примечание:

- ближайший контракт поиска касается только полнотекстового поведения;
- semantic search и embeddings не входят в текущий scope.

### 7.8 Operator endpoints

| Method | Path | Назначение | Доступ | Приоритет |
| --- | --- | --- | --- | --- |
| `GET` | `/api/v1/operator/workspaces` | список workspace для оператора | operator allowlist | later-mvp |
| `GET` | `/api/v1/operator/audit` | глобальный audit | operator allowlist | later-mvp |
| `POST` | `/api/v1/operator/workspaces/{workspaceId}/disable` | временно отключить workspace | operator allowlist | later-mvp |

## 8. Ключевые mutation-пейлоады

### 8.1 Create task

`POST /api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks`

```json
{
  "group_id": "uuid",
  "parent_task_id": null,
  "title": "Describe initial API contract",
  "description_md": "...",
  "priority": "high",
  "rank_key": "a0",
  "assignee_type": "workspace_member",
  "assignee_id": "uuid"
}
```

### 8.2 Update task status

`PATCH /api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/tasks/{taskId}/status`

```json
{
  "status": "in_progress"
}
```

### 8.3 Create note

`POST /api/v1/workspaces/{workspaceSlug}/projects/{projectSlug}/notes`

```json
{
  "kind": "decision",
  "title": "SQLite local profile confirmed",
  "body_md": "...",
  "agent_session_id": null
}
```

### 8.4 Create agent credential

`POST /api/v1/workspaces/{workspaceSlug}/agents/{agentId}/credentials`

```json
{
  "label": "cli-local",
  "project_id": "uuid",
  "scopes": [
    "tasks:read",
    "tasks:write_status",
    "notes:write"
  ],
  "expires_at": null
}
```

Ответ должен единственный раз вернуть полный секрет credential. Повторное чтение полного секрета после выпуска не поддерживается.

## 9. Mapping agent scopes -> HTTP capabilities

| Scope | Разрешенные операции |
| --- | --- |
| `tasks:read` | `GET` task list/detail/dependencies |
| `tasks:write_status` | `PATCH` task status |
| `task_groups:read` | `GET` task group list/detail |
| `documents:read` | `GET` documents |
| `assets:read` | `GET` asset metadata/download |
| `notes:read` | `GET` notes |
| `notes:write` | `POST` notes |
| `audit:read_recent` | `GET` project activity |

## 10. Что идет следующим шагом

- разложить endpoints на реальные Axum handlers и middleware access checks;
- выделить foundation subset, который нужен до полного CRUD;
- синхронизировать payload-поля с миграциями и repository-интерфейсами;
- зафиксировать коды ошибок и доменные validation rules;
- описать upload/download flow для assets после выбора конкретного `AssetStorage` integration flow.