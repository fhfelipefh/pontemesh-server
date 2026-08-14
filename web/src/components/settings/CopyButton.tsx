import { Check, Clipboard } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { IconButton } from "./IconButton";

type CopyButtonProps = {
  value: string;
  label: string;
  className?: string;
};

export function CopyButton({ value, label, className }: CopyButtonProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  async function handleCopy() {
    try {
      await navigator.clipboard?.writeText(value);
      setCopied(true);
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      timeoutRef.current = setTimeout(() => {
        setCopied(false);
      }, 2000);
    } catch {
      // Ignore clipboard write failures gracefully
    }
  }

  const copiedText = t("setup.common.copied", { defaultValue: "Copied!" });
  const currentLabel = copied ? `${label} - ${copiedText}` : label;

  return (
    <IconButton
      className={className}
      label={currentLabel}
      icon={
        copied ? (
          <Check size={15} aria-hidden="true" />
        ) : (
          <Clipboard size={15} aria-hidden="true" />
        )
      }
      onClick={handleCopy}
    />
  );
}
