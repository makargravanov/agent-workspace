import { Download, File, FileImage, Pencil, Plus, Save, Trash2, X } from 'lucide-react';
import type { ChangeEvent, FormEvent } from 'react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import { assetDownloadUrl } from '../../api/assets';
import type { AssetDetail } from '../../api/types';
import {
  useAssets,
  useCreateAsset,
  useDeleteAsset,
  useUpdateAsset,
} from '../../hooks/useAssets';
import { useSession } from '../../hooks/useSession';
import { getErrorMessage } from '../../shared/lib/errors';
import { formatDateTime } from '../../shared/lib/text';
import { FullPageMessage } from '../../shared/ui/FullPageMessage';

type DraftAsset = {
  file: File;
  contentBase64: string;
};

export function AssetsPage() {
  const { workspaceSlug = '', projectSlug = '' } = useParams();
  const sessionQuery = useSession();
  const assetsQuery = useAssets(workspaceSlug, projectSlug);
  const createAssetMutation = useCreateAsset(workspaceSlug, projectSlug);
  const updateAssetMutation = useUpdateAsset(workspaceSlug, projectSlug);
  const deleteAssetMutation = useDeleteAsset(workspaceSlug, projectSlug);
  const [draftAsset, setDraftAsset] = useState<DraftAsset | null>(null);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [fileName, setFileName] = useState('');
  const [mediaType, setMediaType] = useState('');
  const [editingAssetId, setEditingAssetId] = useState<string | null>(null);
  const [editFileName, setEditFileName] = useState('');
  const [editMediaType, setEditMediaType] = useState('');
  const canEdit = canEditAssets(sessionQuery.data?.actor?.role);
  const assets = useMemo(() => assetsQuery.data?.items ?? [], [assetsQuery.data?.items]);
  const totalSize = assets.reduce((sum, asset) => sum + asset.size_bytes, 0);
  const imageCount = assets.filter((asset) => asset.media_type.startsWith('image/')).length;

  async function handleFileChange(event: ChangeEvent<HTMLInputElement>) {
    setUploadError(null);
    const file = event.target.files?.[0] ?? null;
    if (!file) {
      setDraftAsset(null);
      return;
    }

    try {
      const contentBase64 = await readFileAsBase64(file);
      setDraftAsset({ file, contentBase64 });
      setFileName(file.name);
      setMediaType(file.type || 'application/octet-stream');
    } catch (error) {
      setDraftAsset(null);
      setUploadError(getErrorMessage(error));
    }
  }

  function handleUpload(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setUploadError(null);

    if (!draftAsset) {
      setUploadError('Выберите файл для загрузки.');
      return;
    }

    createAssetMutation.mutate(
      {
        file_name: fileName.trim(),
        media_type: mediaType.trim() || 'application/octet-stream',
        content_base64: draftAsset.contentBase64,
      },
      {
        onSuccess: () => {
          setDraftAsset(null);
          setFileName('');
          setMediaType('');
        },
      },
    );
  }

  function startEditing(asset: AssetDetail) {
    setEditingAssetId(asset.id);
    setEditFileName(asset.file_name);
    setEditMediaType(asset.media_type);
  }

  function saveMetadata(asset: AssetDetail) {
    updateAssetMutation.mutate(
      {
        assetId: asset.id,
        payload: {
          file_name: editFileName.trim(),
          media_type: editMediaType.trim() || 'application/octet-stream',
        },
      },
      {
        onSuccess: () => {
          setEditingAssetId(null);
        },
      },
    );
  }

  function deleteAsset(asset: AssetDetail) {
    if (!window.confirm(`Удалить файл «${asset.file_name}»? Это действие необратимо.`)) {
      return;
    }

    deleteAssetMutation.mutate(asset.id);
  }

  if (assetsQuery.isLoading) {
    return <FullPageMessage title="Загрузка файлов" embedded />;
  }

  if (assetsQuery.error) {
    return (
      <FullPageMessage
        title="Не удалось загрузить файлы"
        description={getErrorMessage(assetsQuery.error)}
        embedded
      />
    );
  }

  return (
    <section className="assetsPage">
      <header className="documentsSectionHeader">
        <div className="documentsSectionTitle">
          <p className="documentsEyebrow">Assets</p>
          <h2>Файлы проекта</h2>
          <p className="mutedText">
            Загружаемые артефакты, изображения и вложения, доступные через asset storage.
          </p>
        </div>
      </header>

      <div className="assetStats">
        <article className="statCard">
          <span className="statValue">{assets.length}</span>
          <span className="statLabel">Файлов</span>
        </article>
        <article className="statCard">
          <span className="statValue">{formatBytes(totalSize)}</span>
          <span className="statLabel">Общий размер</span>
        </article>
        <article className="statCard">
          <span className="statValue">{imageCount}</span>
          <span className="statLabel">Изображений</span>
        </article>
      </div>

      {canEdit ? (
        <section className="composePanel assetUploadPanel">
          <div className="compactTitle">
            <Plus size={16} />
            <h2>Загрузить файл</h2>
          </div>

          <form className="formGrid formGridWide" onSubmit={handleUpload}>
            <label className="field fieldSpan2 assetFilePicker">
              <span>Файл</span>
              <input type="file" onChange={handleFileChange} />
            </label>
            <label className="field">
              <span>Имя файла</span>
              <input
                value={fileName}
                onChange={(event) => setFileName(event.target.value)}
                placeholder="brief.pdf"
                required
              />
            </label>
            <label className="field">
              <span>Media type</span>
              <input
                value={mediaType}
                onChange={(event) => setMediaType(event.target.value)}
                placeholder="application/pdf"
                required
              />
            </label>
            <div className="assetUploadSummary">
              {draftAsset ? (
                <>
                  <strong>{draftAsset.file.name}</strong>
                  <span>{formatBytes(draftAsset.file.size)}</span>
                </>
              ) : (
                <span className="mutedText">Файл не выбран</span>
              )}
            </div>
            <div className="formActions">
              <button
                type="submit"
                className="primaryButton compactButton"
                disabled={!draftAsset || createAssetMutation.isPending}
              >
                {createAssetMutation.isPending ? 'Загрузка...' : 'Загрузить файл'}
              </button>
            </div>
          </form>

          {uploadError ? <p className="errorText">{uploadError}</p> : null}
          {createAssetMutation.error ? (
            <p className="errorText">{getErrorMessage(createAssetMutation.error)}</p>
          ) : null}
        </section>
      ) : null}

      <section className="tablePanel assetTablePanel">
        <table className="taskTable assetTable">
          <thead>
            <tr>
              <th>Файл</th>
              <th>Media type</th>
              <th>Размер</th>
              <th>Загрузил</th>
              <th>Создан</th>
              <th>Действия</th>
            </tr>
          </thead>
          <tbody>
            {assets.map((asset) => {
              const isEditing = editingAssetId === asset.id;
              return (
                <tr key={asset.id}>
                  <td>
                    <div className="assetNameCell">
                      {asset.media_type.startsWith('image/') ? (
                        <FileImage size={17} />
                      ) : (
                        <File size={17} />
                      )}
                      {isEditing ? (
                        <input
                          value={editFileName}
                          onChange={(event) => setEditFileName(event.target.value)}
                          aria-label="Имя файла"
                        />
                      ) : (
                        <div>
                          <strong>{asset.file_name}</strong>
                          <span>{asset.sha256 ? asset.sha256.slice(0, 16) : asset.storage_backend}</span>
                        </div>
                      )}
                    </div>
                  </td>
                  <td>
                    {isEditing ? (
                      <input
                        value={editMediaType}
                        onChange={(event) => setEditMediaType(event.target.value)}
                        aria-label="Media type"
                      />
                    ) : (
                      <span className="statusPill">{asset.media_type}</span>
                    )}
                  </td>
                  <td>{formatBytes(asset.size_bytes)}</td>
                  <td>{asset.uploaded_by_member_id ?? 'system'}</td>
                  <td>{formatDateTime(asset.created_at)}</td>
                  <td>
                    <div className="tableActionsCell">
                      <a
                        className="iconButton compactIconButton"
                        href={assetDownloadUrl(workspaceSlug, projectSlug, asset.id)}
                        title="Скачать"
                        aria-label={`Скачать ${asset.file_name}`}
                      >
                        <Download size={14} />
                      </a>
                      {canEdit ? (
                        isEditing ? (
                          <>
                            <button
                              type="button"
                              className="iconButton compactIconButton"
                              onClick={() => saveMetadata(asset)}
                              disabled={updateAssetMutation.isPending}
                              title="Сохранить"
                              aria-label={`Сохранить ${asset.file_name}`}
                            >
                              <Save size={14} />
                            </button>
                            <button
                              type="button"
                              className="iconButton compactIconButton"
                              onClick={() => setEditingAssetId(null)}
                              title="Отменить"
                              aria-label="Отменить редактирование"
                            >
                              <X size={14} />
                            </button>
                          </>
                        ) : (
                          <>
                            <button
                              type="button"
                              className="iconButton compactIconButton"
                              onClick={() => startEditing(asset)}
                              title="Переименовать"
                              aria-label={`Переименовать ${asset.file_name}`}
                            >
                              <Pencil size={14} />
                            </button>
                            <button
                              type="button"
                              className="iconButton dangerIconButton compactIconButton"
                              onClick={() => deleteAsset(asset)}
                              disabled={deleteAssetMutation.isPending}
                              title="Удалить"
                              aria-label={`Удалить ${asset.file_name}`}
                            >
                              <Trash2 size={14} />
                            </button>
                          </>
                        )
                      ) : null}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {assets.length === 0 ? <div className="emptyPanel assetsEmpty">Файлов пока нет.</div> : null}
      </section>

      {updateAssetMutation.error ? (
        <p className="errorText">{getErrorMessage(updateAssetMutation.error)}</p>
      ) : null}
      {deleteAssetMutation.error ? (
        <p className="errorText">{getErrorMessage(deleteAssetMutation.error)}</p>
      ) : null}
    </section>
  );
}

function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = String(reader.result ?? '');
      resolve(result.includes(',') ? result.slice(result.indexOf(',') + 1) : result);
    };
    reader.onerror = () => reject(reader.error ?? new Error('Не удалось прочитать файл.'));
    reader.readAsDataURL(file);
  });
}

function formatBytes(size: number): string {
  if (!Number.isFinite(size) || size <= 0) {
    return '0 B';
  }

  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(Math.floor(Math.log(size) / Math.log(1024)), units.length - 1);
  const value = size / 1024 ** index;
  return `${value >= 10 || index === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[index]}`;
}

function canEditAssets(role: string | undefined): boolean {
  return role === 'owner' || role === 'editor';
}
