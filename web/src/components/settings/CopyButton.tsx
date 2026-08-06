import { Clipboard } from "lucide-react";
import { IconButton } from "./IconButton";

type CopyButtonProps = {
  value: string;
  label: string;
};

export function CopyButton({ value, label }: CopyButtonProps) {
  async function handleCopy() {
    await navigator.clipboard?.writeText(value);
  }

  return (
    <IconButton label={label} icon={<Clipboard size={15} aria-hidden="true" />} onClick={handleCopy} />
  );
}
