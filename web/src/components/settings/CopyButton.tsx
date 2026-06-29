import { Clipboard } from "lucide-react";

type CopyButtonProps = {
  value: string;
  label: string;
};

export function CopyButton({ value, label }: CopyButtonProps) {
  async function handleCopy() {
    await navigator.clipboard?.writeText(value);
  }

  return (
    <button className="settings-copy-button" type="button" title={label} aria-label={label} onClick={handleCopy}>
      <Clipboard size={15} aria-hidden="true" />
    </button>
  );
}
