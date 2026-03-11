# Database Schema: agent-workspace

## 1. Назначение

Этот документ фиксирует текстовую схему БД для этапа Domain foundation.

Цель документа:

- дать один согласованный источник для первых миграций;
- зафиксировать границы между production-oriented PostgreSQL и упрощенным SQLite local/dev профилем;
- описать таблицы, ограничения и индексы до начала CRUD-реализации.

Документ описывает именно первую рабочую схему. Он не пытается заранее покрыть все будущие поисковые, аналитические или интеграционные расширения.

## 2. Стратегия хранения

- каноничный backend для production, shared-dev и self-hosted сценариев: PostgreSQL;
- облегченный backend для local/dev, одиночных запусков и части тестов: SQLite;
- прикладная логика должна работать через persistence/repository-слой, а не напрямую через SQL конкретной СУБД;
- первая схема не должна зависеть от `pgvector`, extension-ов PostgreSQL или отдельного embeddings pipeline;
- SQLite-профиль не обязан обеспечивать полную parity по поиску и эксплуатационным возможностям с PostgreSQL;
- если позже появятся Postgres-only возможности, они должны быть явно помечены как недоступные в SQLite-профиле.

## 3. Общие правила моделирования

- все идентификаторы сущностей логически являются UUID;
- в PostgreSQL UUID хранится как `uuid`, в SQLite как `TEXT` c UUID-строкой;
- все timestamps хранятся в UTC;
- в PostgreSQL timestamps хранятся как `timestamptz`, в SQLite как `TEXT` в ISO-8601/RFC3339 формате;
- enum-поля хранятся как `TEXT` с проверкой на уровне приложения и, где возможно, через `CHECK`-ограничения;
- все slug-поля нормализуются в lowercase kebab-case на уровне приложения;
- поля с JSON-структурой хранятся как `jsonb` в PostgreSQL и как JSON-строка в `TEXT` в SQLite;
- soft-delete как общий паттерн в первой схеме не вводится; для большинства сущностей достаточно status/lifecycle-полей и аудита.

## 4. Состав схемы первой итерации

В первую схему входят только core-domain таблицы:

- workspace и project границы;
- task/task-group/dependency;
- docs/assets/notes/links;
- agents/credentials/sessions;
- integration connections;
- audit trail.

В первую схему не входят:

- embeddings и любые semantic-search таблицы;
- сырые пошаговые agent logs;
- отдельная постоянная модель `PlatformOperator`;
- специальные materialized search indexes за пределами обычных индексов БД.

## 5. Таблицы

### 5.1 `workspaces`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `slug` | text | no | человекочитаемый идентификатор workspace |
| `name` | text | no | отображаемое название |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения metadata |

Ограничения и индексы:

- PK: `id`
- UQ: `slug`

### 5.2 `workspace_members`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `external_subject` | text | no | стабильный subject внешнего auth provider |
| `display_name` | text | no | имя в UI |
| `role` | text | no | `owner|editor|viewer` |
| `status` | text | no | `active|invited|disabled` |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`
- UQ: `(workspace_id, external_subject)`
- INDEX: `(workspace_id, role, status)`

### 5.3 `projects`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `slug` | text | no | slug проекта внутри workspace |
| `name` | text | no | отображаемое имя |
| `status` | text | no | `active|on_hold|archived` |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`
- UQ: `(workspace_id, slug)`
- INDEX: `(workspace_id, status)`

### 5.4 `task_groups`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `kind` | text | no | `initiative|epic` |
| `title` | text | no | название группы |
| `description_md` | text | yes | markdown-описание |
| `status` | text | no | `draft|active|done|archived` |
| `priority` | integer | no | числовая сортировка |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`
- INDEX: `(project_id, status)`
- INDEX: `(project_id, kind, priority)`

### 5.5 `tasks`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `group_id` | uuid | yes | FK -> `task_groups.id` |
| `parent_task_id` | uuid | yes | FK -> `tasks.id` |
| `rank_key` | text | no | ключ стабильной сортировки |
| `starts_at` | timestamp | yes | плановое начало |
| `due_at` | timestamp | yes | плановый дедлайн |
| `assignee_type` | text | yes | `workspace_member|agent` |
| `assignee_id` | uuid | yes | id назначенного субъекта |
| `title` | text | no | заголовок задачи |
| `description_md` | text | yes | описание в markdown |
| `status` | text | no | `todo|in_progress|done|cancelled` |
| `priority` | text | no | `low|normal|high|critical` |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `group_id`, `parent_task_id`
- INDEX: `(project_id, status, priority)`
- INDEX: `(project_id, group_id, rank_key)`
- INDEX: `(project_id, assignee_type, assignee_id)`
- INDEX: `(project_id, parent_task_id)`

Примечание:

- одна задача может принадлежать не более чем одной `TaskGroup`;
- состояние `blocked` в таблице не хранится и вычисляется из `task_dependencies`.

### 5.6 `task_dependencies`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `predecessor_task_id` | uuid | no | FK -> `tasks.id` |
| `successor_task_id` | uuid | no | FK -> `tasks.id` |
| `dependency_type` | text | no | `blocks` |
| `is_hard_block` | boolean | no | жесткая блокировка или нет |
| `created_at` | timestamp | no | момент создания |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `predecessor_task_id`, `successor_task_id`
- UQ: `(project_id, predecessor_task_id, successor_task_id, dependency_type)`
- CHECK: `predecessor_task_id <> successor_task_id`
- INDEX: `(project_id, successor_task_id)`
- INDEX: `(project_id, predecessor_task_id)`

### 5.7 `documents`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `parent_document_id` | uuid | yes | FK -> `documents.id` |
| `slug` | text | no | slug документа внутри родителя или проекта |
| `title` | text | no | заголовок |
| `body_format` | text | no | `markdown` |
| `body_md` | text | no | markdown-содержимое |
| `status` | text | no | `draft|published|archived` |
| `version` | integer | no | текущая версия |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `parent_document_id`
- UQ: `(project_id, parent_document_id, slug)`
- INDEX: `(project_id, status)`
- INDEX: `(project_id, parent_document_id)`

### 5.8 `assets`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `uploaded_by_member_id` | uuid | yes | FK -> `workspace_members.id` |
| `file_name` | text | no | имя файла |
| `media_type` | text | no | MIME type |
| `size_bytes` | bigint | no | размер |
| `sha256` | text | yes | контрольная сумма |
| `storage_backend` | text | no | `local|s3` |
| `storage_key` | text | no | ключ объекта в backend storage |
| `created_at` | timestamp | no | момент загрузки |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `uploaded_by_member_id`
- INDEX: `(project_id, file_name)`
- INDEX: `(project_id, media_type)`
- INDEX: `(project_id, created_at)`
- INDEX: `(project_id, sha256)`

### 5.9 `notes`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `agent_session_id` | uuid | yes | FK -> `agent_sessions.id` |
| `kind` | text | no | `context|worklog|decision|result` |
| `author_type` | text | no | `workspace_member|agent|integration` |
| `author_id` | uuid | no | id автора |
| `title` | text | yes | краткий заголовок |
| `body_md` | text | no | markdown-текст заметки |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `agent_session_id`
- INDEX: `(project_id, kind, created_at)`
- INDEX: `(project_id, author_type, author_id)`
- INDEX: `(project_id, agent_session_id)`

### 5.10 `links`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `source_type` | text | no | внутренний тип источника |
| `source_id` | uuid | no | id источника |
| `target_type` | text | no | внутренний или внешний тип цели |
| `target_id` | uuid | yes | id цели, если цель внутренняя |
| `target_url` | text | yes | URL, если цель внешняя |
| `label` | text | yes | подпись ссылки |
| `created_at` | timestamp | no | момент создания |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`
- INDEX: `(project_id, source_type, source_id)`
- INDEX: `(project_id, target_type, target_id)`

### 5.11 `agents`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `created_by_member_id` | uuid | no | FK -> `workspace_members.id` |
| `key` | text | no | стабильный машинный ключ |
| `display_name` | text | no | отображаемое имя |
| `status` | text | no | `active|disabled` |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `created_by_member_id`
- UQ: `(workspace_id, key)`
- INDEX: `(workspace_id, status)`

### 5.12 `agent_credentials`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | yes | FK -> `projects.id` |
| `agent_id` | uuid | no | FK -> `agents.id` |
| `issued_by_member_id` | uuid | no | FK -> `workspace_members.id` |
| `label` | text | no | имя credential |
| `secret_prefix` | text | no | безопасно отображаемый префикс |
| `secret_hash` | text | no | невосстановимый hash секрета |
| `scope_policy` | json | no | список выданных scopes |
| `status` | text | no | `active|revoked` |
| `expires_at` | timestamp | yes | срок действия |
| `last_used_at` | timestamp | yes | последнее использование |
| `created_at` | timestamp | no | момент выпуска |
| `revoked_at` | timestamp | yes | момент отзыва |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `agent_id`, `issued_by_member_id`
- UQ: `(workspace_id, secret_prefix)`
- INDEX: `(agent_id, status)`
- INDEX: `(workspace_id, project_id, status)`
- INDEX: `(workspace_id, expires_at)`

Примечание:

- в MVP прямой `AgentCredential` является конечным средством аутентификации; отдельный token-exchange не вводится.

### 5.13 `agent_sessions`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `agent_id` | uuid | no | FK -> `agents.id` |
| `status` | text | no | `running|completed|failed|cancelled` |
| `started_at` | timestamp | no | момент старта |
| `finished_at` | timestamp | yes | момент завершения |
| `summary_text` | text | yes | агрегированный итог сессии |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `agent_id`
- INDEX: `(project_id, status, started_at)`
- INDEX: `(agent_id, started_at)`

### 5.14 `agent_session_tasks`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | no | FK -> `projects.id` |
| `agent_session_id` | uuid | no | FK -> `agent_sessions.id` |
| `task_id` | uuid | no | FK -> `tasks.id` |
| `relation_type` | text | no | `primary_context|touched|created|updated` |
| `created_at` | timestamp | no | момент фиксации связи |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `agent_session_id`, `task_id`
- UQ: `(agent_session_id, task_id, relation_type)`
- INDEX: `(project_id, task_id)`
- INDEX: `(project_id, agent_session_id)`

### 5.15 `integration_connections`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | yes | FK -> `projects.id` |
| `provider` | text | no | `github` |
| `scope_kind` | text | no | `workspace|project` |
| `status` | text | no | `active|disabled|error` |
| `config_json` | json | yes | несекретные настройки подключения |
| `secret_ciphertext` | text | yes | зашифрованные секреты |
| `last_synced_at` | timestamp | yes | момент последней синхронизации |
| `created_at` | timestamp | no | момент создания |
| `updated_at` | timestamp | no | момент последнего изменения |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`
- INDEX: `(workspace_id, provider, status)`
- INDEX: `(project_id, provider, status)`

Примечание:

- в MVP GitHub здесь выступает только как integration/mirror, а не как источник истины для внутренних задач.

### 5.16 `audit_events`

| Поле | Тип | Null | Описание |
| --- | --- | --- | --- |
| `id` | uuid | no | PK |
| `workspace_id` | uuid | no | FK -> `workspaces.id` |
| `project_id` | uuid | yes | FK -> `projects.id` |
| `agent_session_id` | uuid | yes | FK -> `agent_sessions.id` |
| `actor_type` | text | no | `workspace_member|agent|integration` |
| `actor_id` | uuid | yes | id автора события |
| `entity_type` | text | no | тип затронутой сущности |
| `entity_id` | uuid | yes | id затронутой сущности |
| `event_type` | text | no | тип события |
| `payload_json` | json | yes | агрегированные metadata изменения |
| `occurred_at` | timestamp | no | момент события |

Ограничения и индексы:

- PK: `id`
- FK: `workspace_id`, `project_id`, `agent_session_id`
- INDEX: `(workspace_id, occurred_at)`
- INDEX: `(project_id, occurred_at)`
- INDEX: `(entity_type, entity_id, occurred_at)`
- INDEX: `(actor_type, actor_id, occurred_at)`

## 6. Правила совместимости PostgreSQL и SQLite

- UUID, timestamps и JSON должны проходить через один типизированный persistence-слой;
- SQL-миграции могут быть раздельными по backend-ам, если это упрощает поддержку различий DDL;
- reference-поведение и полнота ограничений определяются PostgreSQL-профилем;
- SQLite-профиль предназначен для локальной разработки, smoke-тестов и одиночных сценариев, а не для полной эксплуатационной parity;
- если какой-то use case не поддерживается SQLite-профилем, API и сервисный слой должны сообщать это явно.

## 7. Что идет следующим шагом после этой схемы

- подготовить первый набор миграций для PostgreSQL;
- определить минимально нужный набор миграций для SQLite local/dev профиля;
- зафиксировать mapping логических типов в `sqlx`;
- реализовать persistence/repository traits для core-domain;
- синхронизировать HTTP API-контракт с реальными create/read/update сценариями.