import { InputHTMLAttributes } from "react";

type TextInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> & {
  label: string;
  onChange: (value: string) => void;
};

export function TextInput({ id, label, onChange, ...props }: TextInputProps) {
  return (
    <label className="field" htmlFor={id}>
      <span>{label}</span>
      <input
        id={id}
        onChange={(event) => onChange(event.target.value)}
        {...props}
      />
    </label>
  );
}
