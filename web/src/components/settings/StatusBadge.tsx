type StatusBadgeProps = {
  active: boolean;
  activeLabel: string;
  revokedLabel: string;
};

export function StatusBadge({ active, activeLabel, revokedLabel }: StatusBadgeProps) {
  return (
    <span className="settings-status-badge" data-active={active}>
      {active ? activeLabel : revokedLabel}
    </span>
  );
}
