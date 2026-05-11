import { ArrowLeft, Eye, FilePenLine, Plus, RefreshCcw, Save, Trash2 } from 'lucide-react';
import type { FormEvent, ReactNode } from 'react';
import { useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import 'highlight.js/styles/github.css';
import ReactMarkdown, { type Components } from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import remarkGfm from 'remark-gfm';
import type {
  CreateDocumentPayload,
  DocumentDetail,
  DocumentStatus,
  UpdateDocumentPayload,
} from '../../api/types';
import {
  useCreateDocument,
  useDeleteDocument,
  useDocument,
  useDocuments,
  useUpdateDocument,
} from '../../hooks/useDocuments';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { documentStatusLabel, formatDateTime, slugify } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

const DEFAULT_DOCUMENT_STATUS: DocumentStatus = 'draft';

export function DocumentsIndexPage() {
  const navigate = useNavigate();
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const sessionQuery = useSession();
  const documentsQuery = useDocuments(workspaceSlug, projectSlug);
  const canEdit = canEditDocuments(sessionQuery.data?.actor?.role);
  const documents = useMemo(() => documentsQuery.data?.items ?? [], [documentsQuery.data?.items]);
  const tree = useMemo(() => buildDocumentTree(documents), [documents]);

  if (documentsQuery.isLoading) {
    return <FullPageMessage title="Загрузка документов" embedded />;
  }

  if (documentsQuery.error) {
    return (
      <FullPageMessage
        title="Не удалось загрузить документы"
        description={getErrorMessage(documentsQuery.error)}
        embedded
      />
    );
  }

  return (
    <section className="documentsHubPage">
      <section className="documentLibraryHero">
        <div>
          <p className="documentsEyebrow">Knowledge Base</p>
          <h2>Документы проекта</h2>
          <p className="mutedText">
            Страницы знаний, спецификации, runbook&apos;и и справочные материалы проекта.
          </p>
        </div>
        {canEdit ? (
          <button
            type="button"
            className="primaryButton"
            onClick={() =>
              navigate(`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/new`)
            }
          >
            <Plus size={16} />
            <span>Создать документ</span>
          </button>
        ) : null}
      </section>

      <section className="documentsHubLayout">
        <aside className="documentsSidebar documentsSidebarCard">
          <div className="panelHeader">
            <h3>Дерево документов</h3>
            <span className="mutedText">{documents.length}</span>
          </div>
          <div className="documentsTree">
            {tree.map((item) => (
              <DocumentTreeLink
                key={item.document.id}
                node={item}
                workspaceSlug={workspaceSlug}
                projectSlug={projectSlug}
              />
            ))}
          </div>
          {documents.length === 0 ? <div className="emptyPanel">Документов пока нет</div> : null}
        </aside>

        <section className="documentsCatalog">
          {documents.map((document) => (
            <article key={document.id} className="documentCatalogCard">
              <div className="documentCatalogMeta">
                <span className={`statusPill status-${document.status}`}>
                  {documentStatusLabel(document.status)}
                </span>
                <span className="mutedText">{formatDateTime(document.updated_at)}</span>
              </div>
              <div className="documentCatalogBody">
                <div>
                  <h3>{document.title}</h3>
                  <p className="mutedText">{document.slug}</p>
                </div>
                <p className="documentCatalogExcerpt">{summarizeDocument(document.body_md)}</p>
              </div>
              <div className="rowActions">
                <Link
                  className="secondaryButton compactButton"
                  to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${document.id}`}
                >
                  <Eye size={15} />
                  <span>Открыть</span>
                </Link>
                {canEdit ? (
                  <Link
                    className="secondaryButton compactButton"
                    to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${document.id}/edit`}
                  >
                    <FilePenLine size={15} />
                    <span>Редактировать</span>
                  </Link>
                ) : null}
              </div>
            </article>
          ))}
          {documents.length === 0 ? (
            <div className="emptyPanel">
              База знаний ещё пустая. Создайте первую страницу для спецификации или справки.
            </div>
          ) : null}
        </section>
      </section>
    </section>
  );
}

export function DocumentViewPage() {
  const { workspaceSlug = '', projectSlug = '', documentId = '' } = useParams();
  const sessionQuery = useSession();
  const documentQuery = useDocument(workspaceSlug, projectSlug, documentId);
  const canEdit = canEditDocuments(sessionQuery.data?.actor?.role);

  if (documentQuery.isLoading) {
    return <FullPageMessage title="Загрузка документа" embedded />;
  }

  if (documentQuery.error || !documentQuery.data) {
    return (
      <FullPageMessage
        title="Документ не найден"
        description={documentQuery.error ? getErrorMessage(documentQuery.error) : undefined}
        embedded
      />
    );
  }

  const document = documentQuery.data;

  return (
    <section className="documentViewPage">
      <div className="documentPageToolbar">
        <Link
          className="secondaryButton compactButton"
          to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`}
        >
          <ArrowLeft size={15} />
          <span>К списку</span>
        </Link>
        {canEdit ? (
          <Link
            className="primaryButton compactButton"
            to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${document.id}/edit`}
          >
            <FilePenLine size={15} />
            <span>Редактировать</span>
          </Link>
        ) : null}
      </div>

      <article className="documentPageCanvas">
        <header className="documentPageHeader">
          <div className="documentPageHeaderMain">
            <p className="documentsEyebrow">{document.slug}</p>
            <h2>{document.title}</h2>
            <div className="documentPageMetaLine">
              <span className={`statusPill status-${document.status}`}>
                {documentStatusLabel(document.status)}
              </span>
              <span>Обновлён {formatDateTime(document.updated_at)}</span>
              <span>Версия {document.version}</span>
            </div>
          </div>
        </header>

        <div className="documentPageContent">
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            rehypePlugins={[rehypeHighlight]}
            components={markdownComponents}
          >
            {document.body_md}
          </ReactMarkdown>
        </div>
      </article>
    </section>
  );
}

export function CreateDocumentPage() {
  const navigate = useNavigate();
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const sessionQuery = useSession();
  const createDocumentMutation = useCreateDocument(workspaceSlug, projectSlug);
  const canEdit = canEditDocuments(sessionQuery.data?.actor?.role);

  return (
    <DocumentEditorPageShell
      title="Новый документ"
      backTo={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`}
    >
      <DocumentEditor
        mode="create"
        canEdit={canEdit}
        document={null}
        isPending={createDocumentMutation.isPending}
        error={createDocumentMutation.error}
        onSubmit={(payload) => {
          if ('version' in payload) {
            return;
          }
          createDocumentMutation.mutate(payload, {
            onSuccess: (created) => {
              navigate(
                `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${created.id}`,
                { replace: true },
              );
            },
          });
        }}
        onCancel={() => navigate(`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`)}
      />
    </DocumentEditorPageShell>
  );
}

export function EditDocumentPage() {
  const navigate = useNavigate();
  const { workspaceSlug = '', projectSlug = '', documentId = '' } = useParams();
  const sessionQuery = useSession();
  const documentQuery = useDocument(workspaceSlug, projectSlug, documentId);
  const updateDocumentMutation = useUpdateDocument(workspaceSlug, projectSlug, documentId);
  const deleteDocumentMutation = useDeleteDocument(workspaceSlug, projectSlug);
  const canEdit = canEditDocuments(sessionQuery.data?.actor?.role);

  if (documentQuery.isLoading) {
    return <FullPageMessage title="Загрузка документа" embedded />;
  }

  if (documentQuery.error || !documentQuery.data) {
    return (
      <FullPageMessage
        title="Документ не найден"
        description={documentQuery.error ? getErrorMessage(documentQuery.error) : undefined}
        embedded
      />
    );
  }

  const document = documentQuery.data;

  return (
    <DocumentEditorPageShell
      title={`Редактирование: ${document.title}`}
      backTo={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${document.id}`}
    >
      <DocumentEditor
        mode="edit"
        canEdit={canEdit}
        document={document}
        isPending={updateDocumentMutation.isPending}
        error={updateDocumentMutation.error ?? deleteDocumentMutation.error}
        onSubmit={(payload) => updateDocumentMutation.mutate(payload as UpdateDocumentPayload)}
        onDelete={
          canEdit
            ? () => {
                if (!window.confirm(`Удалить документ «${document.title}»? Это действие необратимо.`)) {
                  return;
                }
                deleteDocumentMutation.mutate(document.id, {
                  onSuccess: () => {
                    navigate(`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`, {
                      replace: true,
                    });
                  },
                });
              }
            : undefined
        }
      />
    </DocumentEditorPageShell>
  );
}

function DocumentEditorPageShell({
  title,
  backTo,
  children,
}: {
  title: string;
  backTo: string;
  children: ReactNode;
}) {
  return (
    <section className="documentEditorPage">
      <div className="documentPageToolbar">
        <Link className="secondaryButton compactButton" to={backTo}>
          <ArrowLeft size={15} />
          <span>Назад</span>
        </Link>
      </div>
      <section className="documentEditorPageHeader">
        <p className="documentsEyebrow">Editor</p>
        <h2>{title}</h2>
      </section>
      {children}
    </section>
  );
}

function DocumentTreeLink({
  node,
  workspaceSlug,
  projectSlug,
}: {
  node: DocumentTreeNode;
  workspaceSlug: string;
  projectSlug: string;
}) {
  return (
    <div className="documentTreeNode">
      <Link
        className="documentTreeRow"
        style={{ paddingLeft: `${12 + node.depth * 14}px` }}
        to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${node.document.id}`}
      >
        <div>
          <strong>{node.document.title}</strong>
          <span>{node.document.slug}</span>
        </div>
        <span className={`statusPill status-${node.document.status}`}>
          {documentStatusLabel(node.document.status)}
        </span>
      </Link>
      {node.children.map((child) => (
        <DocumentTreeLink
          key={child.document.id}
          node={child}
          workspaceSlug={workspaceSlug}
          projectSlug={projectSlug}
        />
      ))}
    </div>
  );
}

export function DocumentEditor({
  mode,
  document,
  canEdit,
  onSubmit,
  onDelete,
  onCancel,
  isPending,
  error,
}: {
  mode: 'create' | 'edit';
  document: DocumentDetail | null;
  canEdit: boolean;
  onSubmit: (payload: CreateDocumentPayload | UpdateDocumentPayload) => void;
  onDelete?: () => void;
  onCancel?: () => void;
  isPending: boolean;
  error: unknown;
}) {
  const [title, setTitle] = useState(document?.title ?? '');
  const [slug, setSlug] = useState(document?.slug ?? '');
  const [bodyMd, setBodyMd] = useState(document?.body_md ?? '');
  const [status, setStatus] = useState<DocumentStatus>(document?.status ?? DEFAULT_DOCUMENT_STATUS);
  const [slugEdited, setSlugEdited] = useState(Boolean(document?.slug));
  const [version] = useState(document?.version ?? 1);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canEdit) {
      return;
    }

    if (mode === 'create') {
      onSubmit({
        slug: slug.trim(),
        title: title.trim(),
        body_md: bodyMd,
        status,
      });
      return;
    }

    onSubmit({
      version,
      slug: slug.trim(),
      title: title.trim(),
      body_md: bodyMd,
      status,
    });
  }

  return (
    <section className="composePanel documentEditor documentEditorShell">
      {!canEdit ? (
        <div className="emptyPanel">
          Только просмотр. Для изменения документа нужны права editor или owner.
        </div>
      ) : (
        <div className="documentEditorLayout">
          <form className="documentEditorPane documentEditorFormPane" onSubmit={handleSubmit}>
            <div className="panelHeader">
              <h3>{mode === 'create' ? 'Поля документа' : 'Редактирование'}</h3>
              <span className={`statusPill status-${status}`}>{documentStatusLabel(status)}</span>
            </div>

            <div className="formGrid formGridWide">
              <label className="field">
                <span>Название</span>
                <input
                  value={title}
                  onChange={(event) => {
                    const nextValue = event.target.value;
                    setTitle(nextValue);
                    if (!slugEdited) {
                      setSlug(slugify(nextValue));
                    }
                  }}
                  placeholder="Руководство по проекту"
                  required
                />
              </label>
              <label className="field">
                <span>Slug</span>
                <input
                  value={slug}
                  onChange={(event) => {
                    setSlug(event.target.value);
                    setSlugEdited(true);
                  }}
                  placeholder="project-guide"
                  required
                />
              </label>
              <label className="field fieldSpan2">
                <span>Markdown body</span>
                <textarea
                  className="documentEditorTextarea"
                  value={bodyMd}
                  onChange={(event) => setBodyMd(event.target.value)}
                  rows={22}
                  placeholder="# Документ"
                  required
                />
              </label>
              <label className="field">
                <span>Status</span>
                <select
                  value={status}
                  onChange={(event) => setStatus(event.target.value as DocumentStatus)}
                >
                  <option value="draft">Черновик</option>
                  <option value="published">Опубликован</option>
                  <option value="archived">Архив</option>
                </select>
              </label>
              {mode === 'edit' ? (
                <label className="field">
                  <span>Version</span>
                  <input value={version} readOnly />
                </label>
              ) : null}
            </div>

            <div className="formActions documentEditorActions">
              <button type="submit" className="primaryButton compactButton" disabled={isPending}>
                <Save size={16} />
                <span>{isPending ? 'Сохранение...' : mode === 'create' ? 'Создать' : 'Сохранить'}</span>
              </button>
              {mode === 'edit' && onDelete ? (
                <button
                  type="button"
                  className="iconButton dangerIconButton"
                  onClick={onDelete}
                  disabled={isPending}
                >
                  <Trash2 size={16} />
                  <span>Удалить</span>
                </button>
              ) : null}
              {mode === 'create' && onCancel ? (
                <button
                  type="button"
                  className="secondaryButton compactButton"
                  onClick={onCancel}
                >
                  Отмена
                </button>
              ) : null}
            </div>
          </form>

          <aside className="documentEditorPane documentEditorPreviewPane">
            <div className="panelHeader">
              <div>
                <h3>Preview</h3>
                <p className="mutedText">Живой preview без лишнего chrome, ближе к wiki-странице.</p>
              </div>
              <span className={`statusPill status-${status}`}>{documentStatusLabel(status)}</span>
            </div>

            <div className="documentPreviewMeta">
              <div>
                <span className="statLabel">Название</span>
                <strong>{title || 'Без названия'}</strong>
              </div>
              <div>
                <span className="statLabel">Slug</span>
                <strong>{slug || '—'}</strong>
              </div>
              <div>
                <span className="statLabel">Version</span>
                <strong>{version}</strong>
              </div>
            </div>

            <article className="documentPageCanvas documentPageCanvasPreview">
              <header className="documentPageHeader">
                <div className="documentPageHeaderMain">
                  <p className="documentsEyebrow">{slug || 'draft-document'}</p>
                  <h2>{title || 'Без названия'}</h2>
                </div>
              </header>
              <div className="documentPageContent">
                <ReactMarkdown
                  remarkPlugins={[remarkGfm]}
                  rehypePlugins={[rehypeHighlight]}
                  components={markdownComponents}
                >
                  {bodyMd || '*Пустой документ*'}
                </ReactMarkdown>
              </div>
            </article>
          </aside>
        </div>
      )}

      {hasConflict(error) ? (
        <div className="actionBanner errorBanner documentConflict">
          <div>
            <strong>Конфликт версии.</strong>
            <p>Документ уже изменён на сервере. Перезагрузите страницу и повторите сохранение.</p>
          </div>
          <button
            type="button"
            className="secondaryButton compactButton"
            onClick={() => window.location.reload()}
          >
            <RefreshCcw size={16} />
            <span>Обновить</span>
          </button>
        </div>
      ) : null}

      {error ? <p className="errorText">{getErrorMessage(error)}</p> : null}
    </section>
  );
}

function hasConflict(error: unknown): boolean {
  return Boolean(
    error &&
      typeof error === 'object' &&
      'statusCode' in error &&
      (error as { statusCode?: number }).statusCode === 409,
  );
}

function canEditDocuments(role: string | undefined): boolean {
  return role === 'owner' || role === 'editor';
}

type DocumentTreeNode = {
  document: DocumentDetail;
  depth: number;
  children: DocumentTreeNode[];
};

function buildDocumentTree(documents: DocumentDetail[]): DocumentTreeNode[] {
  const childrenByParent = new Map<string, DocumentDetail[]>();
  const roots: DocumentDetail[] = [];

  for (const document of documents) {
    if (document.parent_document_id) {
      const list = childrenByParent.get(document.parent_document_id) ?? [];
      list.push(document);
      childrenByParent.set(document.parent_document_id, list);
    } else {
      roots.push(document);
    }
  }

  const byCreatedAt = (a: DocumentDetail, b: DocumentDetail) =>
    new Date(b.created_at).getTime() - new Date(a.created_at).getTime();

  roots.sort(byCreatedAt);
  for (const list of childrenByParent.values()) {
    list.sort(byCreatedAt);
  }

  const buildNode = (document: DocumentDetail, depth: number): DocumentTreeNode => ({
    document,
    depth,
    children: (childrenByParent.get(document.id) ?? []).map((child) => buildNode(child, depth + 1)),
  });

  return roots.map((document) => buildNode(document, 0));
}

function summarizeDocument(bodyMd: string): string {
  const normalized = bodyMd
    .replace(/[#>*`[\]-]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  if (normalized.length <= 180) {
    return normalized || 'Без описания';
  }

  return `${normalized.slice(0, 177)}...`;
}

const markdownComponents: Components = {
  h1: ({ children, ...props }) => (
    <h1 className="markdownHeading markdownHeading1" {...props}>
      {children}
    </h1>
  ),
  h2: ({ children, ...props }) => (
    <h2 className="markdownHeading markdownHeading2" {...props}>
      {children}
    </h2>
  ),
  h3: ({ children, ...props }) => (
    <h3 className="markdownHeading markdownHeading3" {...props}>
      {children}
    </h3>
  ),
  p: ({ children, ...props }) => (
    <p className="markdownParagraph" {...props}>
      {children}
    </p>
  ),
  ul: ({ children, ...props }) => (
    <ul className="markdownList" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }) => (
    <ol className="markdownList markdownOrderedList" {...props}>
      {children}
    </ol>
  ),
  blockquote: ({ children, ...props }) => (
    <blockquote className="markdownQuote" {...props}>
      {children}
    </blockquote>
  ),
  hr: (props) => <hr className="markdownDivider" {...props} />,
  pre: ({ children, ...props }) => (
    <pre className="markdownCodeBlock" {...props}>
      {children}
    </pre>
  ),
  code: ({ children, className, ...props }) => (
    <code className={className?.length ? className : 'markdownInlineCode'} {...props}>
      {children}
    </code>
  ),
  a: ({ children, ...props }) => (
    <a {...props} target="_blank" rel="noreferrer">
      {children}
    </a>
  ),
  table: ({ children, ...props }) => (
    <div className="markdownTableWrap">
      <table className="markdownTable" {...props}>
        {children}
      </table>
    </div>
  ),
  th: ({ children, ...props }) => (
    <th className="markdownTableHeader" {...props}>
      {children}
    </th>
  ),
  td: ({ children, ...props }) => (
    <td className="markdownTableCell" {...props}>
      {children}
    </td>
  ),
};
