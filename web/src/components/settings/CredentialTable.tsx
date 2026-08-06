import { ReactNode } from "react";

export type CredentialTableColumn = {
  key: string;
  label?: string;
  ariaLabel?: string;
  className?: string;
};

type CredentialTableProps = {
  columns: CredentialTableColumn[];
  children: ReactNode;
  minWidth?: number;
};

export function CredentialTable({ columns, children, minWidth = 900 }: CredentialTableProps) {
  return (
    <div className="settings-table-wrap">
      <table className="settings-table" style={{ minWidth }}>
        <colgroup>
          {columns.map((column) => (
            <col key={column.key} className={column.className} />
          ))}
        </colgroup>
        <thead>
          <tr>
            {columns.map((column) => (
              <th key={column.key} aria-label={column.ariaLabel}>
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}
