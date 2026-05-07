type FullPageMessageProps = {
  title: string;
  description?: string;
  embedded?: boolean;
};

export function FullPageMessage({ title, description, embedded = false }: FullPageMessageProps) {
  return (
    <main className={embedded ? 'embeddedMessage' : 'fullPageMessage'}>
      <section className="messageCard">
        <h1>{title}</h1>
        {description ? <p className="mutedText">{description}</p> : null}
      </section>
    </main>
  );
}
