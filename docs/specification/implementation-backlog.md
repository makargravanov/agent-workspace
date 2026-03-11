# Implementation Backlog: current API contract

## 1. Назначение

Этот документ раскладывает текущий API-контракт на backlog задач так, чтобы реализацию можно было вести с минимальным количеством блокировок и пересечений.

Scope документа:

- foundation и mvp endpoints из [api-contract.md](./api-contract.md);
- схема данных из [database-schema.md](./database-schema.md);
- текущая архитектурная стратегия modular monolith из [product-spec.md](./product-spec.md).

Вне ближайшего scope:

- semantic search, embeddings и `pgvector`;
- полноценный operator contour из `later-mvp`;
- отдельные диаграммы.

## 2. Правила декомпозиции

Чтобы backlog действительно был слабо связанным, надо заранее принять несколько правил.

### 2.1 Что замораживается заранее

- path и payload shape из API-контракта;
- error envelope и формат `request_id`;
- правила pagination;
- auth контуры: human session, agent bearer credential, operator allowlist;
- общая стратегия persistence: `sqlx` + repository/persistence abstraction + PostgreSQL/SQLite.

### 2.2 Что не должно расползаться по фичам

- ad-hoc HTTP helpers;
- разрозненные SQL-запросы без общего persistence-слоя;
- независимые трактовки audit events;
- случайные изменения contract shape в feature-задачах.

### 2.3 Что включено в каждую backlog-задачу

Если отдельно не оговорено иное, каждая доменная задача включает:

- backend handlers и service-слой;
- repository/persistence implementation для своего домена;
- минимальные contract tests или endpoint smoke tests;
- frontend API client integration для соответствующих endpoint-ов;
- dev fixtures или seed-путь, достаточный для ручной проверки.

### 2.4 Главный источник блокировок

Самая конфликтная точка здесь не handlers, а shared-поверхности:

- миграции;
- auth/access middleware;
- shared DTO/error model;
- router composition;
- audit emission policy.

Поэтому их надо закрыть в самом начале, а не распараллеливать сразу.

## 3. Общий порядок запуска

Рекомендуемый порядок такой:

1. Небольшой foundation sprint для shared-поверхностей.
2. После него параллельный запуск 4 треков: auth/workspace, task domain, knowledge domain, admin/agents.
3. Затем search и consolidated activity/audit read-model.
4. Operator endpoints оставить отдельно, не смешивать с core MVP.

## 4. Foundation backlog

Это короткий обязательный слой, который надо сделать до массового распараллеливания.

| ID | Задача | Scope | Зависимости | Результат |
| --- | --- | --- | --- | --- |
| `BL-00` | Backend composition root | Разбить API на доменные модули и общий router composition; убрать дальнейший рост одного bootstrap-файла | нет | Явные backend-модули под bounded contexts и единая точка сборки роутов |
| `BL-01` | Persistence and migrations foundation | `sqlx`, config, pool management, transaction helper, migration runner для PostgreSQL и SQLite | `BL-00` | Каркас persistence-слоя и исполняемые миграции для обоих backend-ов |
| `BL-02` | Shared HTTP runtime | `request_id`, error envelope, pagination primitives, auth extractors, actor context, audit write helper | `BL-00` | Общие HTTP-конвенции, на которые могут опираться все доменные handlers |
| `BL-03` | Frontend API foundation | typed client/fetch wrapper, auth/session wrapper, conventions для query/mutation hooks, dev mocks | нет | Общий frontend-клиент без разрозненных `fetch` по фичам |
| `BL-04` | Seed and smoke foundation | dev seed path, test fixtures, базовый smoke flow для PostgreSQL и SQLite | `BL-01`, `BL-02` | Быстрый запуск и проверка доменных треков без ручной подготовки данных |

## 5. Параллельные треки после foundation

После `BL-00`...`BL-04` backlog можно разрезать на почти независимые треки.

### 5.1 Track A: auth and workspace core

| ID | Задача | Основные endpoints | Зависимости | Можно вести параллельно с |
| --- | --- | --- | --- | --- |
| `BL-10` | Human auth and session | `/auth/session`, `/auth/github/start`, `/auth/github/callback`, `/auth/dev/login`, `/auth/logout` | `BL-01`, `BL-02`, `BL-03` | `BL-11`, `BL-20`, `BL-30`, `BL-40` |
| `BL-11` | Workspace and project foundation | `/workspaces`, `/workspaces/{workspaceSlug}`, `/projects`, `/projects/{projectSlug}` | `BL-01`, `BL-02`, `BL-03` | Почти со всеми доменными треками; это главный контекст-слой для остальных задач |

Комментарий:

- `BL-11` лучше завершить как можно раньше, потому что почти весь остальной контракт project-scoped.
- `BL-10` и `BL-11` можно делать одновременно, если access checks строятся поверх общего actor context из `BL-02`.

### 5.2 Track B: task domain

| ID | Задача | Основные endpoints | Зависимости | Можно вести параллельно с |
| --- | --- | --- | --- | --- |
| `BL-20` | Tasks foundation | `/tasks` list/create/detail, `/tasks/{taskId}/status` | `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-30`, `BL-31`, `BL-40`, `BL-50` |
| `BL-21` | Task groups | `/task-groups` list/create/detail/patch | `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-20`, `BL-30`, `BL-40` |
| `BL-22` | Task dependencies and blocked view | `/tasks/{taskId}/dependencies` get/post/delete | `BL-20` | `BL-30`, `BL-31`, `BL-40`, `BL-50` |

Комментарий:

- `BL-20` и `BL-21` можно вести параллельно, потому что `Task.group_id` опционален, но к концу обеих задач должна быть согласована полная валидация foreign keys и ссылочной целостности.
- blocked-state не должен храниться отдельно; он вычисляется поверх `task_dependencies`.

### 5.3 Track C: knowledge domain

| ID | Задача | Основные endpoints | Зависимости | Можно вести параллельно с |
| --- | --- | --- | --- | --- |
| `BL-30` | Notes foundation | `/notes` list/create/detail | `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-20`, `BL-21`, `BL-40` |
| `BL-31` | Documents CRUD | `/documents` list/create/detail/patch | `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-20`, `BL-21`, `BL-40` |
| `BL-32` | Asset storage and asset endpoints | `/assets/uploads`, `/assets/{assetId}`, `/assets/{assetId}/download` | `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-20`, `BL-21`, `BL-40` |

Комментарий:

- `BL-32` должен сразу включать `AssetStorage` abstraction и локальный backend, а не только metadata в БД.
- `BL-31` и `BL-32` можно вести почти независимо, но к интеграции важно закрыть привязку assets к documents через `Link` или editor flow.
- optimistic locking для документов надо внедрять сразу, а не откладывать.

### 5.4 Track D: workspace admin and agents

| ID | Задача | Основные endpoints | Зависимости | Можно вести параллельно с |
| --- | --- | --- | --- | --- |
| `BL-40` | Workspace members admin | `/members`, `/members/invites`, `/members/{memberId}` | `BL-10`, `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-20`, `BL-30`, `BL-31` |
| `BL-41` | Agents and credentials | `/agents`, `/agents/{agentId}`, `/agent-credentials`, `/credentials/revoke` | `BL-10`, `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-20`, `BL-30`, `BL-31` |
| `BL-42` | Integration connections | `/integrations`, `/integrations/github` | `BL-10`, `BL-11`, `BL-01`, `BL-02`, `BL-03` | `BL-20`, `BL-30`, `BL-31` |

Комментарий:

- `BL-41` должен сразу учитывать безопасный lifecycle credential: create, one-time secret reveal, revoke, metadata-only read.
- `BL-42` не должен пытаться сразу делать полноценный GitHub sync; в этом backlog достаточно connection lifecycle и read-only integration skeleton.

## 6. Задачи второй волны

Это задачи, которые лучше запускать после того, как foundation и основные доменные треки уже дают реальные данные.

| ID | Задача | Основные endpoints | Зависимости | Причина второй волны |
| --- | --- | --- | --- | --- |
| `BL-50` | Activity and workspace audit read-model | `/projects/{projectSlug}/activity`, `/workspaces/{workspaceSlug}/audit` | `BL-20`, `BL-30`, `BL-40`, `BL-41`, `BL-42` | Читать audit лог имеет смысл после появления достаточного числа producers |
| `BL-51` | Full-text search | `/projects/{projectSlug}/search` | `BL-20`, `BL-30`, `BL-31`, `BL-32` | Search зависит от уже существующих task/note/document/asset read models |

Комментарий:

- audit write helper должен появиться уже в `BL-02`, но полноценные read endpoints для audit и activity лучше делать позже.
- search сейчас должен оставаться только полнотекстовым, без дополнительного semantic-контура.

## 7. Отдельно от текущего core MVP

| ID | Задача | Основные endpoints | Статус |
| --- | --- | --- | --- |
| `BL-90` | Operator contour | `/operator/workspaces`, `/operator/audit`, `/operator/workspaces/{workspaceId}/disable` | Не запускать, пока не закрыт core MVP |

## 8. Практический порядок реализации

Если нужен не просто список, а реальный порядок старта работ, то он такой:

1. `BL-00` ... `BL-04`
2. `BL-10` и `BL-11`
3. Параллельно запустить `BL-20`, `BL-21`, `BL-30`, `BL-31`, `BL-32`, `BL-40`, `BL-41`, `BL-42`
4. После стабилизации producers запустить `BL-22`, `BL-50`, `BL-51`
5. `BL-90` оставить отдельно

## 9. Что сильнее всего может сломать параллельность

Ниже список проблем, которые заранее стоит не допустить:

1. Несколько задач одновременно меняют общую error model или auth middleware.
2. Каждая доменная задача тащит свою схему миграций без общего merge-порядка.
3. Frontend доменные фичи начинают ходить в API напрямую без общего client layer.
4. Audit пишется по-разному в разных доменах, а потом activity нельзя нормально собрать.
5. Search начинают делать до того, как стабилизировались task, note и document read models.

## 10. Минимальный ready-to-start набор

Если нужно начать прямо сейчас и не распыляться, то first actionable pack такой:

1. `BL-00` Backend composition root
2. `BL-01` Persistence and migrations foundation
3. `BL-02` Shared HTTP runtime
4. `BL-03` Frontend API foundation
5. `BL-11` Workspace and project foundation
6. `BL-20` Tasks foundation
7. `BL-30` Notes foundation

После этого уже появится живой skeleton, на который можно безопасно навешивать остальные домены.