import { useRef, useState } from 'react';
import { useUploadBinary } from '../../api/queries/useUpdate';

interface Props {
  open: boolean;
  onClose: () => void;
}

export function UpdateUploadDialog({ open, onClose }: Props) {
  const fileRef = useRef<HTMLInputElement>(null);
  const [version, setVersion] = useState('');
  const upload = useUploadBinary();

  if (!open) return null;

  const handleUpload = () => {
    const file = fileRef.current?.files?.[0];
    if (!file || !version.trim()) return;

    upload.mutate(
      { file, version: version.trim() },
      {
        onSuccess: () => {
          setVersion('');
          if (fileRef.current) fileRef.current.value = '';
          onClose();
        },
      },
    );
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg p-6 w-full max-w-md shadow-xl">
        <h3 className="text-lg font-semibold mb-4">Upload Agent Binary</h3>

        <div className="space-y-4">
          <div>
            <label className="block text-xs text-[var(--text-secondary)] mb-1">
              Version
            </label>
            <input
              type="text"
              value={version}
              onChange={(e) => setVersion(e.target.value)}
              placeholder="e.g. 0.2.0"
              className="w-full px-3 py-2 bg-[var(--bg-base)] border border-[var(--border-default)] rounded text-sm focus:outline-none focus:border-[var(--accent)]"
            />
          </div>

          <div>
            <label className="block text-xs text-[var(--text-secondary)] mb-1">
              Binary File (.exe)
            </label>
            <input
              ref={fileRef}
              type="file"
              accept=".exe"
              className="w-full text-sm text-[var(--text-secondary)] file:mr-3 file:py-1.5 file:px-3 file:rounded file:border-0 file:text-sm file:bg-[var(--bg-elevated)] file:text-[var(--text-primary)] hover:file:bg-[var(--border-default)]"
            />
          </div>

          {upload.isError && (
            <p className="text-xs text-[#ef4444]">
              Upload failed: {(upload.error as Error).message}
            </p>
          )}
        </div>

        <div className="flex justify-end gap-2 mt-6">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded border border-[var(--border-default)] hover:bg-[var(--bg-elevated)]"
          >
            Cancel
          </button>
          <button
            onClick={handleUpload}
            disabled={upload.isPending || !version.trim()}
            className="px-4 py-2 text-sm rounded bg-[var(--accent)] text-white hover:opacity-90 disabled:opacity-50"
          >
            {upload.isPending ? 'Uploading...' : 'Upload'}
          </button>
        </div>
      </div>
    </div>
  );
}
