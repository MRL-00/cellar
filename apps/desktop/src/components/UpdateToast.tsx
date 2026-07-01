import { Icon } from "./icons";

export function UpdateToast({
  version,
  onUpdate,
  onDismiss,
}: {
  version: string;
  onUpdate: () => void;
  onDismiss: () => void;
}) {
  return (
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className="fixed bottom-4 right-4 z-[90] w-[280px] rounded-[7px] border border-border-default bg-bg-1 p-3 shadow-lg"
    >
      <div className="mb-2 flex items-start gap-2">
        <Icon.sparkles size={14} stroke="var(--accent)" />
        <div className="min-w-0 flex-1">
          <div className="text-[12px] font-semibold text-fg-0">Update available</div>
          <div className="text-[11px] text-fg-2">
            Version {version} is ready to download.
          </div>
        </div>
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss update notification"
          className="icon-btn -mr-1 -mt-0.5"
        >
          <Icon.close size={12} />
        </button>
      </div>
      <button
        type="button"
        onClick={onUpdate}
        className="inline-flex h-[26px] w-full items-center justify-center gap-1 rounded-[4px] bg-accent px-2 text-[11px] font-medium text-accent-fg hover:brightness-[1.07]"
      >
        <Icon.download size={11} />
        <span>Update</span>
      </button>
    </div>
  );
}
