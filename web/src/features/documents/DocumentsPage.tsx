import {
  ArrowLeft,
  ChevronDown,
  ChevronRight,
  Eye,
  FilePenLine,
  Info,
  Plus,
  RefreshCcw,
  Save,
  Trash2,
} from 'lucide-react';
import type { Completion, CompletionContext } from '@codemirror/autocomplete';
import { acceptCompletion, autocompletion } from '@codemirror/autocomplete';
import { markdown } from '@codemirror/lang-markdown';
import { EditorView, keymap, placeholder as editorPlaceholder } from '@codemirror/view';
import CodeMirror from '@uiw/react-codemirror';
import type { DragEvent, FormEvent, ReactNode } from 'react';
import { useMemo, useState } from 'react';
import { Link, useLocation, useNavigate, useParams } from 'react-router-dom';
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
  useMoveDocument,
  useRepairDocumentCycles,
  useUpdateDocument,
} from '../../hooks/useDocuments';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { documentStatusLabel, formatDateTime, slugify } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

const DEFAULT_DOCUMENT_STATUS: DocumentStatus = 'draft';

type DocumentTreeNode = {
  document: DocumentDetail;
  depth: number;
  children: DocumentTreeNode[];
};

type ParentOption = {
  id: string;
  label: string;
};

type LineRange = {
  start: number;
  end: number;
};

type EditorMode = 'write' | 'preview' | 'split';

type CycleInfo = {
  cycleDocumentIds: string[];
  cycleGroups: string[][];
};

export function DocumentsIndexPage() {
  const navigate = useNavigate();
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const sessionQuery = useSession();
  const documentsQuery = useDocuments(workspaceSlug, projectSlug);
  const moveDocumentMutation = useMoveDocument(workspaceSlug, projectSlug);
  const repairCyclesMutation = useRepairDocumentCycles(workspaceSlug, projectSlug);
  const canEdit = canEditDocuments(sessionQuery.data?.actor?.role);
  const documents = useMemo(() => documentsQuery.data?.items ?? [], [documentsQuery.data?.items]);
  const tree = useMemo(() => buildDocumentTree(documents), [documents]);
  const cycleInfo = useMemo(() => detectCycleInfo(documents), [documents]);
  const rootCount = tree.filter((node) => node.depth === 0).length;
  const [draggedDocumentId, setDraggedDocumentId] = useState<string | null>(null);
  const [repairError, setRepairError] = useState<string | null>(null);
  const reparentPendingId = moveDocumentMutation.isPending
    ? moveDocumentMutation.variables?.documentId ?? null
    : null;

  function moveDocument(documentId: string, parentDocumentId: string | null) {
    moveDocumentMutation.mutate({
      documentId,
      targetParentDocumentId: parentDocumentId,
    });
  }

  function liftDocument(documentId: string) {
    const currentDocument = documents.find((item) => item.id === documentId);
    if (!currentDocument) {
      return;
    }

    if (!currentDocument.parent_document_id) {
      return;
    }

    const parentDocument = documents.find((item) => item.id === currentDocument.parent_document_id);
    const blockedIds = new Set([documentId, ...collectDescendantIds(documents, documentId)]);
    const candidateParentId = parentDocument?.parent_document_id ?? null;
    const nextParentId =
      candidateParentId && !blockedIds.has(candidateParentId) ? candidateParentId : null;
    moveDocument(documentId, nextParentId);
  }

  async function repairCycles() {
    setRepairError(null);

    try {
      await repairCyclesMutation.mutateAsync();
      await documentsQuery.refetch();
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
              {canEdit && cycleInfo.cycleDocumentIds.length > 0 ? (
                <button
                  type="button"
                  className="secondaryButton compactButton"
                  onClick={() => void repairCycles()}
                  disabled={repairCyclesMutation.isPending}
                >
                  Починить циклы
                </button>
              ) : null}
            </div>
          </div>

          {cycleInfo.cycleDocumentIds.length > 0 ? (
            <div className="actionBanner errorBanner documentsInlineBanner">
              Обнаружены циклы в структуре документов. Проблемных узлов: {cycleInfo.cycleDocumentIds.length}.
            </div>
          ) : null}

          {canEdit ? (
            <div
              className="documentRootDropZone"
              onDragOver={(event) => event.preventDefault()}
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
                  onLiftDocument={liftDocument}
                  reparentPendingId={reparentPendingId}
                />
              ))}
            </div>
          ) : (
            <div className="emptyPanel">Документов пока нет.</div>
          )}

          {moveDocumentMutation.error ? (
            <p className="errorText">{getErrorMessage(moveDocumentMutation.error)}</p>
          ) : null}
          {repairCyclesMutation.error ? (
            <p className="errorText">{getErrorMessage(repairCyclesMutation.error)}</p>
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
                  to={makeDocumentPath(workspaceSlug, projectSlug, document.id)}
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
  const location = useLocation();
  const { workspaceSlug = '', projectSlug = '', documentId = '' } = useParams();
  const sessionQuery = useSession();
  const documentsQuery = useDocuments(workspaceSlug, projectSlug);
  const documentQuery = useDocument(workspaceSlug, projectSlug, documentId);
  const canEdit = canEditDocuments(sessionQuery.data?.actor?.role);
  const documents = useMemo(() => documentsQuery.data?.items ?? [], [documentsQuery.data?.items]);
  const lineRange = useMemo(() => parseLineHash(location.hash), [location.hash]);
  const document = documentQuery.data ?? null;
  const markdownBody = useMemo(
    () => transformWikiLinks(document?.body_md ?? '', documents, workspaceSlug, projectSlug),
    [document?.body_md, documents, workspaceSlug, projectSlug],
  );
  const markdownComponents = useMemo(
    () => createMarkdownComponents(workspaceSlug, projectSlug),
    [workspaceSlug, projectSlug],
  );
  const sourceLines = useMemo(
    () => buildSourceLineWindow(document?.body_md ?? '', lineRange),
    [document?.body_md, lineRange],
  );

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

  if (!document) {
    return null;
  }

  return (
    <section className="documentViewPage">
      <div className="documentPageFrame">
        <div className="documentPageToolbar">
          <Link className="secondaryButton compactButton" to={`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`}>
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
                  triggerLabel="Move to..."
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
                {markdownBody}
              </ReactMarkdown>
            </div>

            {lineRange ? (
              <section className="documentSourcePanel">
                <div className="documentsPaneHeader">
                  <h3>Source lines</h3>
                  <span className="mutedText">
                    L{lineRange.start}{lineRange.end !== lineRange.start ? `-L${lineRange.end}` : ''}
                  </span>
                </div>
                <div className="documentSourceLines">
                  {sourceLines.map((line) => (
                    <div
                      key={line.number}
                      id={`L${line.number}`}
                      className={`documentSourceLine${line.highlighted ? ' isHighlighted' : ''}`}
                    >
                      <span className="documentSourceLineNumber">{line.number}</span>
                      <code>{line.content || ' '}</code>
                    </div>
                  ))}
                </div>
              </section>
            ) : null}
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
              navigate(makeDocumentPath(workspaceSlug, projectSlug, created.id), { replace: true });
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
      backTo={makeDocumentPath(workspaceSlug, projectSlug, document.id)}
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
  onLiftDocument,
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
  onLiftDocument: (documentId: string) => void;
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
            to={makeDocumentPath(workspaceSlug, projectSlug, node.document.id)}
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
          {canEdit && node.document.parent_document_id ? (
            <button
              type="button"
              className="secondaryButton compactButton documentLiftTrigger"
              onClick={() => onLiftDocument(node.document.id)}
              disabled={reparentPendingId === node.document.id}
            >
              Вверх
            </button>
          ) : null}
        </div>
      </div>

      {reparentPendingId === node.document.id ? <p className="mutedText">Сохранение структуры...</p> : null}

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
              onLiftDocument={onLiftDocument}
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
  const moveDocumentMutation = useMoveDocument(workspaceSlug, projectSlug);
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
  const parentDocument = documents.find((item) => item.id === document.parent_document_id);
  const liftedParentId = parentDocument?.parent_document_id ?? '';

  function handleMove() {
    moveDocumentMutation.mutate(
      {
        documentId: document.id,
        targetParentDocumentId: nextParentId || null,
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
              disabled={documentsQuery.isLoading || moveDocumentMutation.isPending}
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
              disabled={moveDocumentMutation.isPending}
            >
              Применить
            </button>
            <button
              type="button"
              className="secondaryButton compactButton"
              onClick={() => setNextParentId(liftedParentId)}
              disabled={!document.parent_document_id || moveDocumentMutation.isPending}
            >
              На уровень выше
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
          {documentsQuery.error ? <p className="errorText">{getErrorMessage(documentsQuery.error)}</p> : null}
          {moveDocumentMutation.error ? (
            <p className="errorText">{getErrorMessage(moveDocumentMutation.error)}</p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function EditorDocumentTreeItem({
  node,
  workspaceSlug,
  projectSlug,
  currentDocumentId,
}: {
  node: DocumentTreeNode;
  workspaceSlug: string;
  projectSlug: string;
  currentDocumentId: string | null;
}) {
  return (
    <div className="editorTreeItem">
      <Link
        className={`editorTreeLink${node.document.id === currentDocumentId ? ' isCurrent' : ''}`}
        style={{ paddingLeft: `${12 + node.depth * 14}px` }}
        to={makeDocumentPath(workspaceSlug, projectSlug, node.document.id)}
      >
        <strong>{node.document.title}</strong>
        <span>{node.document.slug}</span>
      </Link>
      {node.children.length > 0 ? (
        <div className="editorTreeChildren">
          {node.children.map((child) => (
            <EditorDocumentTreeItem
              key={child.document.id}
              node={child}
              workspaceSlug={workspaceSlug}
              projectSlug={projectSlug}
              currentDocumentId={currentDocumentId}
            />
          ))}
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
  const [helpOpen, setHelpOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [editorMode, setEditorMode] = useState<EditorMode>('split');
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

  const previewBody = useMemo(
    () => transformWikiLinks(bodyMd || '*Пустой документ*', documents, workspaceSlug, projectSlug),
    [bodyMd, documents, projectSlug, workspaceSlug],
  );
  const markdownComponents = useMemo(
    () => createMarkdownComponents(workspaceSlug, projectSlug),
    [workspaceSlug, projectSlug],
  );
  const documentTree = useMemo(() => buildDocumentTree(documents), [documents]);
  const lineCount = useMemo(() => Math.max(1, bodyMd.split('\n').length), [bodyMd]);
  const wikiLinkCompletion = useMemo(
    () => createWikiLinkCompletionSource(documents, document?.id ?? null),
    [document?.id, documents],
  );
  const editorExtensions = useMemo(
    () => [
      markdown(),
      autocompletion({
        override: [wikiLinkCompletion],
        activateOnTyping: true,
        defaultKeymap: true,
      }),
      keymap.of([{ key: 'Tab', run: acceptCompletion }]),
      editorPlaceholder('# Документ'),
      documentEditorTheme,
    ],
    [wikiLinkCompletion],
  );

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
          <div className="documentModeTabs" role="tablist" aria-label="Режим редактора">
            <button
              type="button"
              className={`documentModeTab${editorMode === 'write' ? ' isActive' : ''}`}
              onClick={() => setEditorMode('write')}
            >
              Write
            </button>
            <button
              type="button"
              className={`documentModeTab${editorMode === 'preview' ? ' isActive' : ''}`}
              onClick={() => setEditorMode('preview')}
            >
              Preview
            </button>
            <button
              type="button"
              className={`documentModeTab${editorMode === 'split' ? ' isActive' : ''}`}
              onClick={() => setEditorMode('split')}
            >
              Split
            </button>
          </div>
          <button
            type="button"
            className="secondaryButton compactButton"
            onClick={() => setInspectorOpen((value) => !value)}
          >
            <span>{inspectorOpen ? 'Скрыть свойства' : 'Свойства'}</span>
          </button>
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
        <form
          id={formId}
          className={`documentEditorWorkbench${inspectorOpen ? ' inspectorOpen' : ''} mode-${editorMode}`}
          onSubmit={handleSubmit}
        >
          {inspectorOpen ? (
            <aside className="documentEditorInspector">
              <section className="documentInspectorSection">
                <div className="documentInspectorHeader">
                  <h3>Документы</h3>
                  <span>{documents.length}</span>
                </div>
                <div className="documentInspectorTree">
                  {documentTree.map((node) => (
                    <EditorDocumentTreeItem
                      key={node.document.id}
                      node={node}
                      workspaceSlug={workspaceSlug}
                      projectSlug={projectSlug}
                      currentDocumentId={document?.id ?? null}
                    />
                  ))}
                </div>
              </section>

              <section className="documentInspectorSection">
                <div className="documentInspectorHeader">
                  <h3>Свойства</h3>
                </div>
                <div className="documentInspectorFields">
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
                    <span>Родитель</span>
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
            </aside>
          ) : null}

          {editorMode !== 'preview' ? (
          <section className="documentEditorMainPane">
            <div className="documentEditorSurfaceHeader">
              <div className="documentEditorSurfaceTitle">
                <strong>body.md</strong>
                <span>{lineCount} lines</span>
              </div>
              <div className="documentEditorHintControl">
                <button
                  type="button"
                  className={`editorHelpButton${helpOpen ? ' isActive' : ''}`}
                  onClick={() => setHelpOpen((value) => !value)}
                  aria-expanded={helpOpen}
                  aria-label="Справка по markdown-ссылкам"
                  title="Справка по markdown-ссылкам"
                >
                  <Info size={14} />
                </button>
                {helpOpen ? (
                  <div className="documentEditorHintPopover">
                    <strong>Ссылки на документы</strong>
                    <span>`[[slug]]` или `[[slug|Название ссылки]]`</span>
                    <strong>Ссылки на строки</strong>
                    <span>`[[slug#L12-L18]]`</span>
                    <strong>Автодополнение</strong>
                    <span>`[[` открывает список, `Enter` выбирает, `Tab` заменяет, стрелки двигают выбор.</span>
                  </div>
                ) : null}
              </div>
            </div>

            <div className="documentCodeEditorPane">
              <CodeMirror
                value={bodyMd}
                height="68vh"
                extensions={editorExtensions}
                onChange={(value) => setBodyMd(value)}
                basicSetup={{
                  lineNumbers: true,
                  foldGutter: false,
                  dropCursor: false,
                  allowMultipleSelections: false,
                  highlightActiveLine: false,
                  highlightActiveLineGutter: false,
                  autocompletion: true,
                }}
              />
            </div>
          </section>
          ) : null}

          {editorMode !== 'write' ? (
          <aside className="documentEditorPreviewPane">
            <div className="documentsPaneHeader">
              <h3>Preview</h3>
              <Link
                className="secondaryButton compactButton"
                to={document ? makeDocumentPath(workspaceSlug, projectSlug, document.id) : '#'}
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
                    {previewBody}
                  </ReactMarkdown>
                </div>
              </div>
            </article>
          </aside>
          ) : null}
        </form>
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

const documentEditorTheme = EditorView.theme({
  '&': {
    height: '100%',
    fontSize: '14px',
    backgroundColor: '#ffffff',
  },
  '.cm-scroller': {
    overflow: 'auto',
    fontFamily: '"IBM Plex Mono", "Consolas", monospace',
    lineHeight: '1.55',
  },
  '.cm-content': {
    minHeight: '68vh',
    padding: '14px 18px 14px 0',
    caretColor: '#172b4d',
  },
  '.cm-line': {
    paddingLeft: '16px',
  },
  '.cm-gutters': {
    borderRight: '1px solid #dfe1e6',
    backgroundColor: '#fbfcfe',
    color: '#7a869a',
  },
  '.cm-activeLineGutter': {
    backgroundColor: '#fbfcfe',
  },
  '.cm-tooltip.cm-tooltip-autocomplete': {
    border: '1px solid #c1c7d0',
    borderRadius: '0',
    boxShadow: '0 8px 18px rgba(9, 30, 66, 0.14)',
    backgroundColor: '#ffffff',
  },
  '.cm-tooltip-autocomplete > ul': {
    maxHeight: '280px',
    fontFamily: '"IBM Plex Sans", "Segoe UI", sans-serif',
  },
  '.cm-tooltip-autocomplete > ul > li': {
    padding: '6px 10px',
    borderRadius: '0',
  },
  '.cm-tooltip-autocomplete > ul > li[aria-selected]': {
    backgroundColor: '#deebff',
    color: '#172b4d',
  },
  '.cm-completionDetail': {
    color: '#5e6c84',
  },
});

function createWikiLinkCompletionSource(documents: DocumentDetail[], currentDocumentId: string | null) {
  const options = documents
    .filter((document) => document.id !== currentDocumentId)
    .map<Completion>((document) => ({
      label: document.slug,
      detail: document.title,
      type: 'text',
      apply: `${document.slug}]]`,
    }));

  return (context: CompletionContext) => {
    const lookbehind = context.state.sliceDoc(Math.max(0, context.pos - 200), context.pos);
    const openIndex = lookbehind.lastIndexOf('[[');
    const closeIndex = lookbehind.lastIndexOf(']]');

    if (openIndex === -1 || closeIndex > openIndex) {
      return null;
    }

    const absoluteOpenIndex = context.pos - (lookbehind.length - openIndex);
    const query = lookbehind.slice(openIndex + 2);

    if (query.includes('\n') || query.includes(']') || query.includes('#')) {
      return null;
    }

    const normalizedQuery = query.trim().toLowerCase();
    const filteredOptions = options.filter((option) => {
      if (normalizedQuery.length === 0) {
        return true;
      }

      return (
        option.label.toLowerCase().includes(normalizedQuery) ||
        String(option.detail ?? '').toLowerCase().includes(normalizedQuery)
      );
    });

    if (filteredOptions.length === 0 && !context.explicit) {
      return null;
    }

    return {
      from: absoluteOpenIndex + 2,
      to: context.pos,
      options: filteredOptions,
      validFor: /^[^\]\n#|]*$/,
    };
  };
}

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

  const tree = roots.map((document) => buildNode(document, 0, new Set()));

  const fallbackRoots: DocumentTreeNode[] = [];
  for (const document of [...documents].sort(byCreatedAt)) {
    if (!visited.has(document.id)) {
      fallbackRoots.push(buildNode(document, 0, new Set()));
    }
  }

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

function detectCycleInfo(documents: DocumentDetail[]): CycleInfo {
  const documentById = new Map(documents.map((document) => [document.id, document]));
  const indexById = new Map<string, number>();
  const lowLinkById = new Map<string, number>();
  const stack: string[] = [];
  const onStack = new Set<string>();
  const cycleGroups: string[][] = [];
  const cycleIds = new Set<string>();
  let index = 0;

  function strongConnect(documentId: string) {
    indexById.set(documentId, index);
    lowLinkById.set(documentId, index);
    index += 1;
    stack.push(documentId);
    onStack.add(documentId);

    const parentId = documentById.get(documentId)?.parent_document_id;
    if (parentId && documentById.has(parentId)) {
      if (!indexById.has(parentId)) {
        strongConnect(parentId);
        lowLinkById.set(
          documentId,
          Math.min(lowLinkById.get(documentId) ?? 0, lowLinkById.get(parentId) ?? 0),
        );
      } else if (onStack.has(parentId)) {
        lowLinkById.set(
          documentId,
          Math.min(lowLinkById.get(documentId) ?? 0, indexById.get(parentId) ?? 0),
        );
      }
    }

    if (lowLinkById.get(documentId) !== indexById.get(documentId)) {
      return;
    }

    const component: string[] = [];
    while (stack.length > 0) {
      const currentId = stack.pop();
      if (!currentId) {
        break;
      }
      onStack.delete(currentId);
      component.push(currentId);
      if (currentId === documentId) {
        break;
      }
    }

    const isSelfLoop =
      component.length === 1 && documentById.get(component[0])?.parent_document_id === component[0];
    if (component.length > 1 || isSelfLoop) {
      cycleGroups.push(component);
      for (const id of component) {
        cycleIds.add(id);
      }
    }
  }

  for (const document of documents) {
    if (!indexById.has(document.id)) {
      strongConnect(document.id);
    }
  }

  return {
    cycleDocumentIds: [...cycleIds],
    cycleGroups,
  };
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

function parseLineHash(hash: string): LineRange | null {
  const match = hash.match(/^#L(\d+)(?:-L?(\d+))?$/i);
  if (!match) {
    return null;
  }

  const start = Number(match[1]);
  const end = Number(match[2] ?? match[1]);

  if (!Number.isFinite(start) || !Number.isFinite(end) || start <= 0 || end <= 0) {
    return null;
  }

  return {
    start: Math.min(start, end),
    end: Math.max(start, end),
  };
}

function buildSourceLineWindow(bodyMd: string, range: LineRange | null) {
  const lines = bodyMd.split('\n');
  if (!range) {
    return [];
  }

  const from = Math.max(1, range.start - 3);
  const to = Math.min(lines.length, range.end + 3);

  return lines.slice(from - 1, to).map((content, index) => {
    const lineNumber = from + index;
    return {
      number: lineNumber,
      content,
      highlighted: lineNumber >= range.start && lineNumber <= range.end,
    };
  });
}

function resolveDocumentReference(reference: string, documents: DocumentDetail[]): DocumentDetail | null {
  const normalizedReference = reference.trim().toLowerCase();
  return (
    documents.find((document) => document.id.toLowerCase() === normalizedReference) ??
    documents.find((document) => document.slug.toLowerCase() === normalizedReference) ??
    null
  );
}

function transformWikiLinks(
  bodyMd: string,
  documents: DocumentDetail[],
  workspaceSlug: string,
  projectSlug: string,
): string {
  return bodyMd.replace(/\[\[([^[\]]+)\]\]/g, (fullMatch, rawInner: string) => {
    const [rawTarget, rawLabel] = rawInner.split('|');
    const target = rawTarget.trim();
    const label = rawLabel?.trim();

    if (!target) {
      return fullMatch;
    }

    const [reference, hash = ''] = target.split('#');
    const document = resolveDocumentReference(reference, documents);
    if (!document) {
      return fullMatch;
    }

    const href = makeDocumentPath(
      workspaceSlug,
      projectSlug,
      document.id,
      hash.length > 0 ? `#${hash}` : '',
    );
    const linkLabel = label || document.title;
    return `[${linkLabel}](${href})`;
  });
}

function createMarkdownComponents(
  workspaceSlug: string,
  projectSlug: string,
): Components {
  return {
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
    a: ({ children, href, ...props }) => {
      if (href?.startsWith(`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/`)) {
        return (
          <Link to={href} {...props}>
            {children}
          </Link>
        );
      }

      return (
        <a href={href} {...props} target="_blank" rel="noreferrer">
          {children}
        </a>
      );
    },
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
}

function makeDocumentPath(
  workspaceSlug: string,
  projectSlug: string,
  documentId: string,
  hash = '',
): string {
  return `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${documentId}${hash}`;
}
