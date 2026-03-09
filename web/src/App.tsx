type Highlight = {
  title: string;
  description: string;
};

const highlights: Highlight[] = [
  {
    title: "Единое рабочее пространство",
    description:
      "Задачи, планы, документы и заметки связываются в общую модель и не живут разрозненно по разным системам.",
  },
  {
    title: "Агенты как полноценные участники",
    description:
      "MCP-утилита и API должны давать агентам контролируемый доступ к контексту, обновлениям планов и журналу изменений.",
  },
  {
    title: "Гибридный поиск",
    description:
      "Точный поиск строится на PostgreSQL full-text search, а семантический слой закладывается через pgvector без отдельного стора на старте.",
  },
];

const milestones = [
  "Зафиксировать MVP, сущности и сценарии совместной работы",
  "Спроектировать API, схему БД и аудит действий",
  "Реализовать базовый цикл задач, заметок и документов",
  "Добавить GitHub-интеграцию и MCP-инструменты для агентов",
];

const components = [
  "Rust API на Axum",
  "Rust MCP-bridge как отдельная локальная утилита",
  "React + TypeScript интерфейс рабочего пространства",
  "PostgreSQL + pgvector для хранения и поиска контекста",
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
        </p>
        <div className="heroBadges">
          <span>Rust backend</span>
          <span>React + TypeScript</span>
          <span>PostgreSQL + pgvector</span>
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
