import {
  ArrowLeft,
  ChevronDown,
  ChevronRight,
  Eye,
  FilePenLine,
  Plus,
  RefreshCcw,
  Save,
  Trash2,
} from 'lucide-react';
import type { DragEvent, FormEvent, ReactNode } from 'react';
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
  useReparentDocument,
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
  const reparentDocumentMutation = useReparentDocument(workspaceSlug, projectSlug);
  const canEdit = canEditDocuments(sessionQuery.data?.actor?.role);
  const documents = useMemo(() => documentsQuery.data?.items ?? [], [documentsQuery.data?.items]);
  const tree = useMemo(() => buildDocumentTree(documents), [documents]);
  const cycleDocumentIds = useMemo(() => detectCycleDocumentIds(documents), [documents]);
  const rootCount = tree.length;
  const [draggedDocumentId, setDraggedDocumentId] = useState<string | null>(null);
  const [repairError, setRepairError] = useState<string | null>(null);

  function moveDocument(documentId: string, parentDocumentId: string | null) {
    const currentDocument = documents.find((item) => item.id === documentId);
    if (!currentDocument) {
      return;
    }

    reparentDocumentMutation.mutate({
      documentId,
      payload: {
        version: currentDocument.version,
        parent_document_id: parentDocumentId,
      },
    });
  }

  async function repairCycles() {
    setRepairError(null);

    try {
      for (const documentId of cycleDocumentIds) {
        const currentDocument = documents.find((item) => item.id === documentId);
        if (!currentDocument || currentDocument.parent_document_id === null) {
          continue;
        }

        await reparentDocumentMutation.mutateAsync({
          documentId,
          payload: {
            version: currentDocument.version,
            parent_document_id: null,
          },
        });
      }
    } catch (error) {
      setRepairError(getErrorMessage(error));
    }
  }

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
    <section className="documentsIndexPage">
      <header className="documentsSectionHeader">
        <div className="documentsSectionTitle">
          <p className="documentsEyebrow">Knowledge base</p>
          <h2>Документы проекта</h2>
          <p className="mutedText">
            Спецификации, runbook&apos;и и справочные страницы в иерархии проекта.
          </p>
        </div>
        {canEdit ? (
          <button
            type="button"
            className="primaryButton compactButton"
            onClick={() =>
              navigate(`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/new`)
            }
          >
            <Plus size={16} />
            <span>Создать документ</span>
          </button>
        ) : null}
      </header>

      <section className="documentsIndexLayout">
        <div className="documentsTreePane">
          <div className="documentsPaneHeader">
            <h3>Дерево документов</h3>
            <div className="rowActions">
              <span className="mutedText">{documents.length}</span>
              {canEdit && cycleDocumentIds.length > 0 ? (
                <button
                  type="button"
                  className="secondaryButton compactButton"
                  onClick={() => void repairCycles()}
                  disabled={reparentDocumentMutation.isPending}
                >
                  Починить циклы
                </button>
              ) : null}
            </div>
          </div>

          {cycleDocumentIds.length > 0 ? (
            <div className="actionBanner errorBanner documentsInlineBanner">
              Обнаружены циклы в структуре документов. Проблемных узлов: {cycleDocumentIds.length}.
            </div>
          ) : null}

          {canEdit ? (
            <div
              className="documentRootDropZone"
              onDragOver={(event) => {
                event.preventDefault();
              }}
              onDrop={(event) => {
                event.preventDefault();
                const documentId = event.dataTransfer.getData('text/document-id') || draggedDocumentId;
                setDraggedDocumentId(null);
                if (!documentId) {
                  return;
                }
                const currentDocument = documents.find((item) => item.id === documentId);
                if (!currentDocument || currentDocument.parent_document_id === null) {
                  return;
                }
                moveDocument(documentId, null);
              }}
            >
              Перетащите сюда, чтобы сделать документ корневым
            </div>
          ) : null}

          {tree.length > 0 ? (
            <div className="documentsTreeList">
              {tree.map((node) => (
                <DocumentTreeItem
                  key={node.document.id}
                  node={node}
                  documents={documents}
                  workspaceSlug={workspaceSlug}
                  projectSlug={projectSlug}
                  canEdit={canEdit}
                  draggedDocumentId={draggedDocumentId}
                  setDraggedDocumentId={setDraggedDocumentId}
                  onMoveDocument={moveDocument}
                  reparentPendingId={reparentDocumentMutation.variables?.documentId ?? null}
                />
              ))}
            </div>
          ) : (
            <div className="emptyPanel">Документов пока нет.</div>
          )}
          {reparentDocumentMutation.error ? (
            <p className="errorText">{getErrorMessage(reparentDocumentMutation.error)}</p>
          ) : null}
          {repairError ? <p className="errorText">{repairError}</p> : null}
        </div>

        <aside className="documentsIndexSummary">
          <div className="documentsPaneHeader">
            <h3>Каталог</h3>
          </div>
          <dl className="documentsFactList">
            <div>
              <dt>Всего документов</dt>
              <dd>{documents.length}</dd>
            </div>
            <div>
              <dt>Корневых страниц</dt>
              <dd>{rootCount}</dd>
            </div>
            <div>
              <dt>Последнее обновление</dt>
              <dd>{documents[0] ? formatDateTime(documents[0].updated_at) : '—'}</dd>
            </div>
          </dl>

          {documents.length > 0 ? (
            <div className="documentsFlatList">
              {documents.slice(0, 6).map((document) => (
                <Link
                  key={document.id}
                  className="documentsFlatRow"
                  to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${document.id}`}
                >
                  <div>
                    <strong>{document.title}</strong>
                    <span>{document.slug}</span>
                  </div>
                  <span className={`statusPill status-${document.status}`}>
                    {documentStatusLabel(document.status)}
                  </span>
                </Link>
              ))}
            </div>
          ) : null}
        </aside>
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
      <div className="documentPageFrame">
        <div className="documentPageToolbar">
          <Link
            className="secondaryButton compactButton"
            to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`}
          >
            <ArrowLeft size={15} />
            <span>К каталогу</span>
          </Link>
          <div className="rowActions">
            {canEdit ? (
              <>
                <DocumentMoveControl
                  workspaceSlug={workspaceSlug}
                  projectSlug={projectSlug}
                  document={document}
                  triggerLabel="Move to…"
                  buttonClassName="secondaryButton compactButton"
                />
                <Link
                  className="primaryButton compactButton"
                  to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${document.id}/edit`}
                >
                  <FilePenLine size={15} />
                  <span>Редактировать</span>
                </Link>
              </>
            ) : null}
          </div>
        </div>

        <article className="documentPageSurface">
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

          <div className="documentPageContent documentPageContentReadable">
            <div className="markdownPreview">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                rehypePlugins={[rehypeHighlight]}
                components={markdownComponents}
              >
                {document.body_md}
              </ReactMarkdown>
            </div>
          </div>
        </article>
      </div>
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
      title={document.title}
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
      <div className="documentPageFrame">
        <div className="documentPageToolbar">
          <Link className="secondaryButton compactButton" to={backTo}>
            <ArrowLeft size={15} />
            <span>Назад</span>
          </Link>
        </div>

        <header className="documentEditorPageHeader">
          <p className="documentsEyebrow">Editor</p>
          <h2>{title}</h2>
        </header>

        {children}
      </div>
    </section>
  );
}

function DocumentTreeItem({
  node,
  documents,
  workspaceSlug,
  projectSlug,
  canEdit,
  draggedDocumentId,
  setDraggedDocumentId,
  onMoveDocument,
  reparentPendingId,
}: {
  node: DocumentTreeNode;
  documents: DocumentDetail[];
  workspaceSlug: string;
  projectSlug: string;
  canEdit: boolean;
  draggedDocumentId: string | null;
  setDraggedDocumentId: (documentId: string | null) => void;
  onMoveDocument: (documentId: string, parentDocumentId: string | null) => void;
  reparentPendingId: string | null;
}) {
  const [expanded, setExpanded] = useState(true);
  const hasChildren = node.children.length > 0;
  const blockedParentIds = useMemo(
    () => new Set([node.document.id, ...collectDescendantIds(documents, node.document.id)]),
    [documents, node.document.id],
  );

  function handleDrop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    const sourceDocumentId = event.dataTransfer.getData('text/document-id') || draggedDocumentId;
    setDraggedDocumentId(null);

    if (!sourceDocumentId || blockedParentIds.has(sourceDocumentId)) {
      return;
    }

    const sourceDocument = documents.find((item) => item.id === sourceDocumentId);
    if (!sourceDocument || sourceDocument.parent_document_id === node.document.id) {
      return;
    }

    onMoveDocument(sourceDocumentId, node.document.id);
    setExpanded(true);
  }

  return (
    <div className="documentTreeItem">
      <div
        className={`documentTreeRow${draggedDocumentId === node.document.id ? ' isDragging' : ''}`}
        style={{ paddingLeft: `${node.depth * 20}px` }}
        onDragOver={(event) => {
          if (canEdit) {
            event.preventDefault();
          }
        }}
        onDrop={canEdit ? handleDrop : undefined}
      >
        <div className="documentTreeRowMain">
          {hasChildren ? (
            <button
              type="button"
              className="documentTreeToggle"
              onClick={() => setExpanded((value) => !value)}
              aria-label={expanded ? 'Свернуть раздел' : 'Развернуть раздел'}
            >
              {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            </button>
          ) : (
            <span className="documentTreeSpacer" />
          )}

          <Link
            className="documentTreeLink"
            to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${node.document.id}`}
            draggable={canEdit}
            onDragStart={(event) => {
              if (!canEdit) {
                return;
              }
              event.dataTransfer.setData('text/document-id', node.document.id);
              setDraggedDocumentId(node.document.id);
            }}
            onDragEnd={() => setDraggedDocumentId(null)}
          >
            <strong>{node.document.title}</strong>
            <span>{node.document.slug}</span>
          </Link>
        </div>

        <div className="documentTreeRowMeta">
          <span className={`statusPill status-${node.document.status}`}>
            {documentStatusLabel(node.document.status)}
          </span>
          {canEdit ? (
            <DocumentMoveControl
              workspaceSlug={workspaceSlug}
              projectSlug={projectSlug}
              document={node.document}
              triggerLabel="Move"
              buttonClassName="secondaryButton compactButton documentMoveTrigger"
              compact
            />
          ) : null}
        </div>
      </div>

      {reparentPendingId === node.document.id ? <p className="mutedText">Сохранение структуры…</p> : null}

      {hasChildren && expanded ? (
        <div className="documentTreeChildren">
          {node.children.map((child) => (
            <DocumentTreeItem
              key={child.document.id}
              node={child}
              documents={documents}
              workspaceSlug={workspaceSlug}
              projectSlug={projectSlug}
              canEdit={canEdit}
              draggedDocumentId={draggedDocumentId}
              setDraggedDocumentId={setDraggedDocumentId}
              onMoveDocument={onMoveDocument}
              reparentPendingId={reparentPendingId}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function DocumentMoveControl({
  workspaceSlug,
  projectSlug,
  document,
  triggerLabel,
  buttonClassName,
  compact = false,
}: {
  workspaceSlug: string;
  projectSlug: string;
  document: DocumentDetail;
  triggerLabel: string;
  buttonClassName: string;
  compact?: boolean;
}) {
  const documentsQuery = useDocuments(workspaceSlug, projectSlug);
  const reparentDocumentMutation = useReparentDocument(workspaceSlug, projectSlug);
  const documents = useMemo(() => documentsQuery.data?.items ?? [], [documentsQuery.data?.items]);
  const blockedParentIds = useMemo(
    () => new Set([document.id, ...collectDescendantIds(documents, document.id)]),
    [document.id, documents],
  );
  const parentOptions = useMemo(
    () => buildParentOptions(documents, blockedParentIds),
    [documents, blockedParentIds],
  );
  const [isOpen, setIsOpen] = useState(false);
  const [nextParentId, setNextParentId] = useState(document.parent_document_id ?? '');

  function handleMove() {
    reparentDocumentMutation.mutate(
      {
        documentId: document.id,
        payload: {
          version: document.version,
          parent_document_id: nextParentId || null,
        },
      },
      {
        onSuccess: () => {
          setIsOpen(false);
        },
      },
    );
  }

  return (
    <div className={`documentMoveControl${compact ? ' isCompact' : ''}`}>
      <button
        type="button"
        className={buttonClassName}
        onClick={() => setIsOpen((value) => !value)}
      >
        {triggerLabel}
      </button>

      {isOpen ? (
        <div className="documentMovePanel">
          <label className="field">
            <span>Новый родитель</span>
            <select
              value={nextParentId}
              onChange={(event) => setNextParentId(event.target.value)}
              disabled={documentsQuery.isLoading || reparentDocumentMutation.isPending}
            >
              <option value="">Корневой документ</option>
              {parentOptions.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <div className="rowActions">
            <button
              type="button"
              className="primaryButton compactButton"
              onClick={handleMove}
              disabled={reparentDocumentMutation.isPending}
            >
              Применить
            </button>
            <button
              type="button"
              className="secondaryButton compactButton"
              onClick={() => {
                setNextParentId(document.parent_document_id ?? '');
                setIsOpen(false);
              }}
            >
              Отмена
            </button>
          </div>
          {documentsQuery.error ? (
            <p className="errorText">{getErrorMessage(documentsQuery.error)}</p>
          ) : null}
          {reparentDocumentMutation.error ? (
            <p className="errorText">{getErrorMessage(reparentDocumentMutation.error)}</p>
          ) : null}
        </div>
      ) : null}
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
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const documentsQuery = useDocuments(workspaceSlug, projectSlug);
  const documents = useMemo(() => documentsQuery.data?.items ?? [], [documentsQuery.data?.items]);
  const blockedParentIds = useMemo(
    () => new Set(document ? [document.id, ...collectDescendantIds(documents, document.id)] : []),
    [document, documents],
  );
  const parentOptions = useMemo(
    () => buildParentOptions(documents, blockedParentIds),
    [documents, blockedParentIds],
  );
  const [title, setTitle] = useState(document?.title ?? '');
  const [slug, setSlug] = useState(document?.slug ?? '');
  const [bodyMd, setBodyMd] = useState(document?.body_md ?? '');
  const [parentDocumentId, setParentDocumentId] = useState(document?.parent_document_id ?? '');
  const [status, setStatus] = useState<DocumentStatus>(document?.status ?? DEFAULT_DOCUMENT_STATUS);
  const [slugEdited, setSlugEdited] = useState(Boolean(document?.slug));
  const [version] = useState(document?.version ?? 1);
  const formId = `document-editor-${mode}`;

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
        parent_document_id: parentDocumentId || null,
        status,
      });
      return;
    }

    onSubmit({
      version,
      slug: slug.trim(),
      title: title.trim(),
      body_md: bodyMd,
      parent_document_id: parentDocumentId || null,
      status,
    });
  }

  return (
    <section className="documentEditorShell">
      <div className="documentEditorToolbar">
        <div className="documentEditorToolbarTitle">
          <h3>{mode === 'create' ? 'Новый документ' : 'Редактирование'}</h3>
          <div className="documentPageMetaLine">
            <span className={`statusPill status-${status}`}>{documentStatusLabel(status)}</span>
            {mode === 'edit' ? <span>Версия {version}</span> : null}
          </div>
        </div>

        <div className="rowActions">
          <button
            type="submit"
            form={formId}
            className="primaryButton compactButton"
            disabled={isPending || !canEdit}
          >
            <Save size={16} />
            <span>{isPending ? 'Сохранение...' : mode === 'create' ? 'Создать' : 'Сохранить'}</span>
          </button>
          {mode === 'edit' && onDelete ? (
            <button
              type="button"
              className="secondaryButton compactButton dangerButton"
              onClick={onDelete}
              disabled={isPending || !canEdit}
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
      </div>

      {!canEdit ? (
        <div className="emptyPanel">
          Только просмотр. Для изменения документа нужны права editor или owner.
        </div>
      ) : (
        <div className="documentEditorLayout">
          <form id={formId} className="documentEditorFormPane" onSubmit={handleSubmit}>
            <section className="documentEditorSection">
              <div className="documentsPaneHeader">
                <h3>Метаданные</h3>
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
                <label className="field">
                  <span>Родительский документ</span>
                  <select
                    value={parentDocumentId}
                    onChange={(event) => setParentDocumentId(event.target.value)}
                    disabled={documentsQuery.isLoading}
                  >
                    <option value="">Корневой документ</option>
                    {parentOptions.map((option) => (
                      <option key={option.id} value={option.id}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </label>
                {mode === 'edit' ? (
                  <label className="field">
                    <span>Version</span>
                    <input value={version} readOnly />
                  </label>
                ) : null}
              </div>
              {documentsQuery.error ? (
                <p className="warningText">
                  Не удалось загрузить дерево документов: {getErrorMessage(documentsQuery.error)}
                </p>
              ) : null}
            </section>

            <section className="documentEditorSection">
              <div className="documentsPaneHeader">
                <h3>Markdown</h3>
              </div>
              <label className="field">
                <span>Содержимое</span>
                <textarea
                  className="documentEditorTextarea"
                  value={bodyMd}
                  onChange={(event) => setBodyMd(event.target.value)}
                  rows={24}
                  placeholder="# Документ"
                  required
                />
              </label>
            </section>
          </form>

          <aside className="documentEditorPreviewPane">
            <div className="documentsPaneHeader">
              <h3>Preview</h3>
              <Link
                className="secondaryButton compactButton"
                to={
                  document
                    ? `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${document.id}`
                    : '#'
                }
                onClick={(event) => {
                  if (!document) {
                    event.preventDefault();
                  }
                }}
              >
                <Eye size={15} />
                <span>Открыть страницу</span>
              </Link>
            </div>

            <article className="documentPageSurface documentPageSurfacePreview">
              <header className="documentPageHeader">
                <div className="documentPageHeaderMain">
                  <p className="documentsEyebrow">{slug || 'draft-document'}</p>
                  <h2>{title || 'Без названия'}</h2>
                  <div className="documentPageMetaLine">
                    <span className={`statusPill status-${status}`}>
                      {documentStatusLabel(status)}
                    </span>
                    <span>Версия {version}</span>
                  </div>
                </div>
              </header>

              <div className="documentPageContent documentPageContentCompact">
                <div className="markdownPreview">
                  <ReactMarkdown
                    remarkPlugins={[remarkGfm]}
                    rehypePlugins={[rehypeHighlight]}
                    components={markdownComponents}
                  >
                    {bodyMd || '*Пустой документ*'}
                  </ReactMarkdown>
                </div>
              </div>
            </article>
          </aside>
        </div>
      )}

      {hasConflict(error) ? (
        <div className="actionBanner errorBanner documentConflict">
          <div>
            <strong>Конфликт версии.</strong>
            <p>Документ уже изменён на сервере. Обновите страницу и повторите сохранение.</p>
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

type ParentOption = {
  id: string;
  label: string;
};

function buildDocumentTree(documents: DocumentDetail[]): DocumentTreeNode[] {
  const childrenByParent = new Map<string, DocumentDetail[]>();
  const roots: DocumentDetail[] = [];
  const documentById = new Map(documents.map((document) => [document.id, document]));

  for (const document of documents) {
    if (document.parent_document_id && documentById.has(document.parent_document_id)) {
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

  const visited = new Set<string>();

  const buildNode = (document: DocumentDetail, depth: number, lineage: Set<string>): DocumentTreeNode => {
    visited.add(document.id);
    const nextLineage = new Set(lineage);
    nextLineage.add(document.id);

    return {
      document,
      depth,
      children: (childrenByParent.get(document.id) ?? [])
        .filter((child) => !nextLineage.has(child.id))
        .map((child) => buildNode(child, depth + 1, nextLineage)),
    };
  };

  const tree: DocumentTreeNode[] = roots.map((document) => buildNode(document, 0, new Set()));

  const fallbackRoots = documents
    .filter((document) => !visited.has(document.id))
    .sort(byCreatedAt)
    .map((document) => buildNode(document, 0, new Set()));

  return [...tree, ...fallbackRoots];
}

function collectDescendantIds(documents: DocumentDetail[], documentId: string): string[] {
  const childrenByParent = new Map<string, string[]>();

  for (const document of documents) {
    if (!document.parent_document_id) {
      continue;
    }
    const list = childrenByParent.get(document.parent_document_id) ?? [];
    list.push(document.id);
    childrenByParent.set(document.parent_document_id, list);
  }

  const descendants: string[] = [];
  const visited = new Set<string>();
  const queue = [...(childrenByParent.get(documentId) ?? [])];

  while (queue.length > 0) {
    const currentId = queue.shift();
    if (!currentId || visited.has(currentId)) {
      continue;
    }
    visited.add(currentId);
    descendants.push(currentId);
    queue.push(...(childrenByParent.get(currentId) ?? []));
  }

  return descendants;
}

function detectCycleDocumentIds(documents: DocumentDetail[]): string[] {
  const documentById = new Map(documents.map((document) => [document.id, document]));
  const state = new Map<string, 'visiting' | 'visited'>();
  const cycleIds = new Set<string>();

  function visit(documentId: string, path: string[]) {
    const currentState = state.get(documentId);
    if (currentState === 'visited') {
      return;
    }

    if (currentState === 'visiting') {
      const cycleStart = path.indexOf(documentId);
      const cyclePath = cycleStart >= 0 ? path.slice(cycleStart) : [documentId];
      for (const id of cyclePath) {
        cycleIds.add(id);
      }
      return;
    }

    state.set(documentId, 'visiting');
    const document = documentById.get(documentId);
    const parentId = document?.parent_document_id;

    if (parentId && documentById.has(parentId)) {
      visit(parentId, [...path, documentId]);
    }

    state.set(documentId, 'visited');
  }

  for (const document of documents) {
    visit(document.id, []);
  }

  return [...cycleIds];
}

function buildParentOptions(
  documents: DocumentDetail[],
  blockedParentIds: Set<string>,
): ParentOption[] {
  const tree = buildDocumentTree(documents);
  const options: ParentOption[] = [];

  const walk = (nodes: DocumentTreeNode[]) => {
    for (const node of nodes) {
      if (!blockedParentIds.has(node.document.id)) {
        options.push({
          id: node.document.id,
          label: `${'  '.repeat(node.depth)}${node.document.title}`,
        });
      }
      walk(node.children);
    }
  };

  walk(tree);
  return options;
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
