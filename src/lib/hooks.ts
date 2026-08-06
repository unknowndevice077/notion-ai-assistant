import { useEffect } from "react";

/**
 * Closes an open dropdown/popover when the user clicks outside `ref`'s
 * subtree or presses Escape — standard dropdown hygiene that PresetBar and
 * BusinessBar were both missing (they only closed via their own toggle
 * button, so clicking anywhere else on the page left them open forever).
 */
export function useOnClickOutside(ref: React.RefObject<HTMLElement>, active: boolean, onClose: () => void) {
  useEffect(() => {
    if (!active) return;

    const handlePointer = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };

    document.addEventListener("mousedown", handlePointer);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handlePointer);
      document.removeEventListener("keydown", handleKey);
    };
  }, [active, ref, onClose]);
}
