import { CSSProperties, HTMLAttributes, LabelHTMLAttributes, ReactNode } from "react";

type FormSectionProps = HTMLAttributes<HTMLElement> & {
  children: ReactNode;
};

type FormSectionHeaderProps = HTMLAttributes<HTMLDivElement> & {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
};

type ColumnGridProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
  columns?: number;
};

type FormFieldProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
  label: ReactNode;
  htmlFor?: string;
  help?: ReactNode;
  error?: ReactNode;
};

type CheckboxFieldProps = Omit<LabelHTMLAttributes<HTMLLabelElement>, "children"> & {
  children: ReactNode;
  label: ReactNode;
};

export function FormSection({ children, className, ...sectionProps }: FormSectionProps) {
  const classes = ["form-section", className].filter(Boolean).join(" ");

  return (
    <section className={classes} {...sectionProps}>
      {children}
    </section>
  );
}

export function FormSectionHeader({
  title,
  description,
  actions,
  className,
  ...headerProps
}: FormSectionHeaderProps) {
  const classes = ["form-section-header", className].filter(Boolean).join(" ");

  return (
    <div className={classes} {...headerProps}>
      <div className="form-section-header__content">
        <h3>{title}</h3>
        {description ? <p>{description}</p> : null}
      </div>
      {actions ? <div className="form-section-header__actions">{actions}</div> : null}
    </div>
  );
}

export function FormGrid({ children, columns = 2, className, style, ...gridProps }: ColumnGridProps) {
  const classes = ["form-grid", className].filter(Boolean).join(" ");

  return (
    <div
      className={classes}
      style={{ "--form-grid-columns": columns, ...style } as CSSProperties}
      {...gridProps}
    >
      {children}
    </div>
  );
}

export function FormField({
  children,
  label,
  htmlFor,
  help,
  error,
  className,
  ...fieldProps
}: FormFieldProps) {
  const classes = ["form-field", className].filter(Boolean).join(" ");

  return (
    <div className={classes} {...fieldProps}>
      <label htmlFor={htmlFor}>{label}</label>
      {children}
      {help ? <p className="form-field__help">{help}</p> : null}
      {error ? <p className="form-field__error">{error}</p> : null}
    </div>
  );
}

export function CheckboxGrid({ children, columns = 3, className, style, ...gridProps }: ColumnGridProps) {
  const classes = ["checkbox-grid", className].filter(Boolean).join(" ");

  return (
    <div
      className={classes}
      style={{ "--checkbox-grid-columns": columns, ...style } as CSSProperties}
      {...gridProps}
    >
      {children}
    </div>
  );
}

export function CheckboxField({ children, label, className, ...labelProps }: CheckboxFieldProps) {
  const classes = ["checkbox-field", className].filter(Boolean).join(" ");

  return (
    <label className={classes} {...labelProps}>
      {children}
      <span>{label}</span>
    </label>
  );
}

export function ActionBar({ children, className, ...barProps }: HTMLAttributes<HTMLDivElement>) {
  const classes = ["action-bar", className].filter(Boolean).join(" ");

  return (
    <div className={classes} {...barProps}>
      {children}
    </div>
  );
}

export function ButtonGroup({ children, className, ...groupProps }: HTMLAttributes<HTMLDivElement>) {
  const classes = ["button-group", className].filter(Boolean).join(" ");

  return (
    <div className={classes} {...groupProps}>
      {children}
    </div>
  );
}
