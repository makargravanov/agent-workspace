import { Plus, RefreshCcw, Save, Trash2 } from 'lucide-react';
import type { FormEvent } from 'react';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import type { CreateDocumentPayload, DocumentDetail, DocumentStatus, UpdateDocumentPayload } from '../../api/types';
import { useDeleteDocument, useCreateDocument, useDocument, useDocuments, useUpdateDocument } from '../../hooks/useDocuments';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { documentStatusLabel, formatDateTime, slugify } from '../../shared/lib/text';

const DEFAULT_DOCUMENT_STATUS: DocumentStatus = 'draft';

export function DocumentsPage() {
  const navigate = useNavigate();
  const { workspaceSlug = '', projectSlug = '', documentId = '' } = useParams();
  const sessionQuery = useSession();
  const documentsQuery = useDocuments(workspaceSlug, projectSlug);
  const documentQuery = useDocument(workspaceSlug, projectSlug, documentId);
  const createDocumentMutation = useCreateDocument(workspaceSlug, projectSlug);
  const deleteDocumentMutation = useDeleteDocument(workspaceSlug, projectSlug);
  const [isCreating, setIsCreating] = useState(false);

  const documents = useMemo(() => documentsQuery.data?.items ?? [], [documentsQuery.data?.items]);
  const documentMap = useMemo(() => new Map(documents.map((item) => [item.id, item])), [documents]);
  const routeDocument = documentId ? documentQuery.data ?? documentMap.get(documentId) ?? null : null;
  const selectedDocument = routeDocument ?? documents[0] ?? null;
  const updateDocumentMutation = useUpdateDocument(workspaceSlug, projectSlug, selectedDocument?.id ?? '');
  const canEdit = sessionQuery.data?.actor?.role === 'owner' || sessionQuery.data?.actor?.role === 'editor';

  useEffect(() => {
    if (isCreating || !documentId || documentsQuery.isLoading || documentQuery.isLoading) {
      return;
    }

    if (documentQuery.error || (documentId && !documentMap.has(documentId))) {
      const nextDocument = documents.find((item) => item.id !== documentId) ?? null;
      const nextPath = nextDocument
        ? `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${nextDocument.id}`
        : `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`;
      navigate(nextPath, { replace: true });
    }
  }, [
    documentId,
    documentMap,
    documentQuery.error,
    documentQuery.isLoading,
    documents,
    documentsQuery.isLoading,
    isCreating,
    navigate,
    projectSlug,
    workspaceSlug,
  ]);

  if (documentsQuery.isLoading) {
    return <div className="emptyPanel">Загрузка документов...</div>;
  }

  function openCreateMode() {
    setIsCreating(true);
    navigate(`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`, { replace: false });
  }

  function handleCreate(payload: CreateDocumentPayload | UpdateDocumentPayload) {
    if ('version' in payload) {
      return;
    }

    createDocumentMutation.mutate(payload, {
      onSuccess: (created) => {
        setIsCreating(false);
        navigate(
          `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${created.id}`,
          { replace: true },
        );
      },
    });
  }

  function handleDelete(document: DocumentDetail) {
    if (!window.confirm(`Удалить документ «${document.title}»? Это действие необратимо.`)) {
      return;
    }

    deleteDocumentMutation.mutate(document.id, {
      onSuccess: () => {
        const remaining = documents.filter((item) => item.id !== document.id);
        const nextDocument = remaining[0] ?? null;
        const nextPath = nextDocument
          ? `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${nextDocument.id}`
          : `/workspaces/${workspaceSlug}/projects/${projectSlug}/documents`;
        navigate(nextPath, { replace: true });
      },
    });
  }

  return (
    <section className="documentsPage">
      <DocumentList
        documents={documents}
        selectedDocumentId={selectedDocument?.id ?? null}
        onSelect={(nextDocumentId) =>
          navigate(`/workspaces/${workspaceSlug}/projects/${projectSlug}/documents/${nextDocumentId}`)
        }
        onCreateClick={canEdit ? openCreateMode : undefined}
        canEdit={canEdit}
        createPending={createDocumentMutation.isPending}
      />

      <section className="documentsMain">
        {isCreating ? (
          <DocumentEditor
            mode="create"
            canEdit={canEdit}
            document={null}
            onSubmit={handleCreate}
            onCancel={() => setIsCreating(false)}
            isPending={createDocumentMutation.isPending}
            error={createDocumentMutation.error}
          />
        ) : selectedDocument ? (
          <>
            <DocumentPreview document={selectedDocument} />
            <DocumentEditor
              key={selectedDocument.id}
              mode="edit"
              canEdit={canEdit}
              document={selectedDocument}
              onSubmit={(payload) => updateDocumentMutation.mutate(payload as UpdateDocumentPayload)}
              onDelete={canEdit ? () => handleDelete(selectedDocument) : undefined}
              isPending={updateDocumentMutation.isPending}
              error={updateDocumentMutation.error}
            />
          </>
        ) : (
          <div className="emptyPanel">Документов пока нет</div>
        )}

        {documentsQuery.error ? <p className="errorText">{getErrorMessage(documentsQuery.error)}</p> : null}
        {documentQuery.error ? <p className="errorText">{getErrorMessage(documentQuery.error)}</p> : null}
        {deleteDocumentMutation.error ? (
          <p className="errorText">{getErrorMessage(deleteDocumentMutation.error)}</p>
        ) : null}
      </section>
    </section>
  );
}

export function DocumentList({
  documents,
  selectedDocumentId,
  onSelect,
  onCreateClick,
  canEdit,
  createPending,
}: {
  documents: DocumentDetail[];
  selectedDocumentId: string | null;
  onSelect: (documentId: string) => void;
  onCreateClick?: () => void;
  canEdit: boolean;
  createPending: boolean;
}) {
  const tree = useMemo(() => buildDocumentTree(documents), [documents]);

  return (
    <aside className="documentsSidebar">
      <div className="panelHeader">
        <h2>Документы</h2>
        {canEdit && onCreateClick ? (
          <button type="button" className="primaryButton compactButton" onClick={onCreateClick} disabled={createPending}>
            <Plus size={16} />
            <span>Создать документ</span>
          </button>
        ) : null}
      </div>

      <div className="documentsTree">
        {tree.map((item) => (
          <DocumentTreeRow
            key={item.document.id}
            node={item}
            selectedDocumentId={selectedDocumentId}
            onSelect={onSelect}
          />
        ))}
      </div>

      {documents.length === 0 ? <div className="emptyPanel">Документов пока нет</div> : null}
    </aside>
  );
}

function DocumentTreeRow({
  node,
  selectedDocumentId,
  onSelect,
}: {
  node: DocumentTreeNode;
  selectedDocumentId: string | null;
  onSelect: (documentId: string) => void;
}) {
  return (
    <div className="documentTreeNode">
      <button
        type="button"
        className={`documentTreeRow${selectedDocumentId === node.document.id ? ' isActive' : ''}`}
        style={{ paddingLeft: `${12 + node.depth * 14}px` }}
        onClick={() => onSelect(node.document.id)}
      >
        <div>
          <strong>{node.document.title}</strong>
          <span>{node.document.slug}</span>
        </div>
        <span className={`statusPill status-${node.document.status}`}>{documentStatusLabel(node.document.status)}</span>
      </button>
      {node.children.map((child) => (
        <DocumentTreeRow
          key={child.document.id}
          node={child}
          selectedDocumentId={selectedDocumentId}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

export function DocumentPreview({ document }: { document: DocumentDetail }) {
  return (
    <section className="documentPreview">
      <div className="panelHeader">
        <div>
          <h2>{document.title}</h2>
          <p className="mutedText">{document.slug}</p>
        </div>
        <span className={`statusPill status-${document.status}`}>{documentStatusLabel(document.status)}</span>
      </div>

      <div className="documentMetaGrid">
        <div className="statCard">
          <span className="statLabel">Версия</span>
          <strong className="statValue">{document.version}</strong>
        </div>
        <div className="statCard">
          <span className="statLabel">Обновлён</span>
          <strong>{formatDateTime(document.updated_at)}</strong>
        </div>
        <div className="statCard">
          <span className="statLabel">Создан</span>
          <strong>{formatDateTime(document.created_at)}</strong>
        </div>
      </div>

      <article className="documentBodyPreview">
        <div className="compactTitle">
          <RefreshCcw size={16} />
          <h3>Markdown preview</h3>
        </div>
        <pre>{document.body_md}</pre>
      </article>
    </section>
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
  const [slugEdited, setSlugEdited] = useState(false);
  const [version] = useState(document?.version ?? 1);

  const conflict = hasConflict(error);
  const showConflictHint = conflict;

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
    <section className="composePanel documentEditor">
      <div className="panelHeader">
        <div className="compactTitle">
          <Save size={16} />
          <h2>{mode === 'create' ? 'Создать документ' : 'Редактировать документ'}</h2>
        </div>
        {mode === 'edit' ? <span className={`statusPill status-${status}`}>{documentStatusLabel(status)}</span> : null}
      </div>

      {!canEdit ? (
        <div className="emptyPanel">Только просмотр. Для изменения документа нужны права editor или owner.</div>
      ) : (
        <form className="formGrid formGridWide" onSubmit={handleSubmit}>
          <label className="field">
            <span>Название</span>
            <input
              value={title}
              onChange={(event) => {
                const next = event.target.value;
                setTitle(next);
                if (mode === 'create' && !slugEdited) {
                  setSlug(slugify(next));
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
              value={bodyMd}
              onChange={(event) => setBodyMd(event.target.value)}
              rows={14}
              placeholder="# Документ"
              required
            />
          </label>
          <label className="field">
            <span>Status</span>
            <select value={status} onChange={(event) => setStatus(event.target.value as DocumentStatus)}>
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
          <div className="formActions documentEditorActions">
            <button type="submit" className="primaryButton compactButton" disabled={isPending}>
              <Save size={16} />
              <span>{isPending ? 'Сохранение...' : mode === 'create' ? 'Создать' : 'Сохранить'}</span>
            </button>
            {mode === 'edit' && onDelete ? (
              <button type="button" className="iconButton dangerIconButton" onClick={onDelete} disabled={isPending}>
                <Trash2 size={16} />
                <span>Удалить</span>
              </button>
            ) : null}
            {mode === 'create' && onCancel ? (
              <button type="button" className="secondaryButton compactButton" onClick={onCancel}>
                Отмена
              </button>
            ) : null}
          </div>
        </form>
      )}

      {showConflictHint ? (
        <div className="actionBanner errorBanner documentConflict">
          <div>
            <strong>Конфликт версии.</strong>
            <p>Документ уже был изменён на сервере. Обновите его и повторите сохранение.</p>
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
