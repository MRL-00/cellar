import { useConfirm } from "../../state/confirm";
import { Modal } from "./Modal";

const BTN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border px-3 text-[11.5px] font-medium transition-[background,color,border-color,filter] duration-[120ms]";
const BTN_SUBTLE =
  BTN_BASE +
  " text-fg-1 bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0";
const BTN_PRIMARY =
  BTN_BASE + " bg-accent text-accent-fg border-transparent hover:brightness-[1.07]";
const BTN_DANGER =
  BTN_BASE + " bg-warn text-white border-transparent hover:brightness-[1.07]";

/** Singleton confirm modal. Mount once near the app root; driven by
 *  `useConfirm().ask(...)`. */
export function ConfirmDialog() {
  const request = useConfirm((s) => s.request);
  const resolve = useConfirm((s) => s.resolve);
  if (!request) return null;

  return (
    <Modal onClose={() => resolve(false)} width={420}>
      <div className="flex flex-col gap-3 p-4">
        <span className="text-[12.5px] font-semibold text-fg-0">
          {request.title}
        </span>
        <span className="whitespace-pre-line text-[11.5px] leading-relaxed text-fg-2">
          {request.message}
        </span>
        <div className="mt-1 flex justify-end gap-2">
          <button
            type="button"
            className={BTN_SUBTLE}
            onClick={() => resolve(false)}
            autoFocus
          >
            Cancel
          </button>
          <button
            type="button"
            className={request.danger ? BTN_DANGER : BTN_PRIMARY}
            onClick={() => resolve(true)}
          >
            {request.confirmLabel ?? "Confirm"}
          </button>
        </div>
      </div>
    </Modal>
  );
}
