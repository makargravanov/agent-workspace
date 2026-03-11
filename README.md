# agent-workspace

`agent-workspace` — монорепозиторий для системы совместного ведения задач, групп задач, документации и заметок для людей и агентных инструментов.

На текущем этапе репозиторий инициализирован как стартовый каркас с базовой архитектурой:

- Rust API для прикладной логики и интеграций.
- Rust MCP-утилита для подключения IDE-агентов и CLI-агентов.
- React + TypeScript фронтенд для рабочего пространства.
- Docker Compose для локального развертывания.
- PostgreSQL как основной backend данных, с планируемым SQLite local/dev профилем через persistence-слой.

## Стартовые решения

- Архитектурный стиль первой версии: модульный монолит.
- База данных первой версии: PostgreSQL как основной backend и SQLite как упрощенный local/dev профиль.
- Поиск первой версии: полнотекстовый; semantic search и `pgvector` отложены.
- Человеческая аутентификация первой версии: GitHub OAuth как стартовый provider с сохранением пути к OIDC.
- Интеграция с GitHub: закладывается в спецификацию как отдельный bounded context, но не выделяется в отдельный сервис на старте.
- Развертывание: Docker Compose локально, контейнерная сборка для каждого компонента.

## Структура репозитория

```text
docs/                  Спецификации и архитектурные решения
infra/docker/          Dockerfiles для сервисов
services/api/          Rust API
tools/mcp-bridge/      Rust MCP-обертка
web/                   React + TypeScript фронтенд
```

## Документы

- `docs/specification/product-spec.md` — стартовая продуктовая и техническая спецификация.
- `docs/specification/database-schema.md` — текстовая схема БД для этапа Domain foundation.
- `docs/specification/api-contract.md` — первичный HTTP API-контракт.
- `docs/adr/0001-bootstrap-architecture.md` — первичное архитектурное решение по стеку и границам системы.

## Быстрый старт

1. Скопировать переменные окружения из `.env.example` в `.env` при необходимости.
2. Поднять локальное окружение:

   ```bash
   docker compose up --build
   ```

3. Локальный запуск API без Docker:

   ```bash
   cargo run -p agent-workspace-api
   ```

4. Локальный запуск MCP-утилиты без Docker:

   ```bash
   cargo run -p agent-workspace-mcp -- stdio
   ```

5. Локальный запуск фронтенда:

   ```bash
   cd web
   npm install
   npm run dev
   ```

## Следующий этап

- Реализовать migrations и persistence adapter для PostgreSQL и SQLite local/dev профиля.
- Поднять auth/access skeleton для human и agent контуров.
- Реализовать foundation endpoints по workspace, project, task group, task, note и audit.
- Подготовить базовый read-only контур GitHub integration и MCP surface поверх API.
