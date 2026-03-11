type Highlight = {
  title: string;
  description: string;
};

const highlights: Highlight[] = [
  {
    title: "Единое рабочее пространство",
    description:
      "Задачи, группы задач, документы и заметки связываются в общую модель и не живут разрозненно по разным системам.",
  },
  {
    title: "Агенты как полноценные участники",
    description:
      "MCP-утилита и API должны давать агентам контролируемый доступ к контексту, задачам, заметкам и журналу изменений.",
  },
  {
    title: "Сначала полнотекстовый поиск",
    description:
      "Ближайший приоритет - предсказуемый полнотекстовый поиск и прозрачный API-контракт; semantic search и embeddings отложены до стабилизации core-domain.",
  },
];

const milestones = [
  "Зафиксировать схему БД и persistence adapter для Postgres и SQLite",
  "Зафиксировать первичный API-контракт и access model",
  "Реализовать migrations и foundation endpoints",
  "Добавить GitHub read-only integration и MCP-инструменты",
];

const components = [
  "Rust API на Axum",
  "Rust MCP-bridge как отдельная локальная утилита",
  "React + TypeScript интерфейс рабочего пространства",
  "PostgreSQL как основной backend данных + SQLite local/dev профиль",
];

export default function App() {
  return (
    <main className="pageShell">
      <section className="heroPanel">
        <p className="eyebrow">agent-workspace / bootstrap</p>
        <h1>Система совместной работы людей и агентных инструментов</h1>
        <p className="heroCopy">
          Стартовая версия репозитория уже закладывает модульный монолит, Docker-развертывание,
          PostgreSQL как основной источник истины и отдельную MCP-обертку для интеграции с IDE и CLI.
          Ближайший шаг - зафиксировать текстовую схему БД и API-контракт, сохранив упрощенный
          local/dev профиль на SQLite.
        </p>
        <div className="heroBadges">
          <span>Rust backend</span>
          <span>React + TypeScript</span>
          <span>PostgreSQL primary</span>
          <span>SQLite local profile</span>
          <span>Docker-first</span>
        </div>
      </section>

      <section className="highlightGrid" aria-label="Ключевые принципы">
        {highlights.map((item) => (
          <article key={item.title} className="card">
            <h2>{item.title}</h2>
            <p>{item.description}</p>
          </article>
        ))}
      </section>

      <section className="detailLayout">
        <article className="card accentCard">
          <p className="sectionLabel">Стартовая архитектура</p>
          <ul className="listBlock">
            {components.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </article>

        <article className="card roadmapCard">
          <p className="sectionLabel">Ближайшие шаги</p>
          <ol className="timeline">
            {milestones.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ol>
        </article>
      </section>
    </main>
  );
}
