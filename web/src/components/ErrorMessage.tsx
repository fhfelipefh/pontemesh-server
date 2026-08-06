type ErrorMessageProps = {
  message: string;
  id?: string;
};

export function ErrorMessage({ message, id }: ErrorMessageProps) {
  if (!message) {
    return null;
  }

  return (
    <p className="error" id={id} role="alert" aria-live="polite">
      {message}
    </p>
  );
}
