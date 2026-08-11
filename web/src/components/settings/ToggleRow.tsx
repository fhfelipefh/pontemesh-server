type ToggleRowProps = {
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange?: (checked: boolean) => void;
};

export function ToggleRow({ label, checked, disabled = false, onChange }: ToggleRowProps) {
  return (
    <div className="settings-toggle-row">
      <span>{label}</span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        title={label}
        disabled={disabled}
        onClick={() => onChange?.(!checked)}
      >
        <span aria-hidden="true" />
      </button>
    </div>
  );
}
