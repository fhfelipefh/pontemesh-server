import { useMemo, useState } from "react";
import { getTimezoneOptions, TimezoneOption } from "../../utils/timezones";

type TimezoneSelectProps = {
  id?: string;
  label: string;
  help?: string;
  value: string;
  disabled?: boolean;
  onChange: (value: string) => void;
};

export function TimezoneSelect({
  id = "instance-timezone-select",
  label,
  help,
  value,
  disabled = false,
  onChange
}: TimezoneSelectProps) {
  const [filter, setFilter] = useState("");
  const allOptions = useMemo(() => getTimezoneOptions(), []);

  const filteredOptions = useMemo(() => {
    const query = filter.trim().toLowerCase();
    if (!query) {
      return allOptions;
    }
    return allOptions.filter((option) => option.searchKey.includes(query));
  }, [allOptions, filter]);

  // Ensure current value is in the options list
  const optionsToRender = useMemo(() => {
    if (value && !filteredOptions.some((opt) => opt.value === value)) {
      const currentOpt = allOptions.find((opt) => opt.value === value) ?? {
        value,
        label: `${value}`,
        city: value,
        offset: "UTC",
        searchKey: value.toLowerCase()
      };
      return [currentOpt, ...filteredOptions];
    }
    return filteredOptions;
  }, [allOptions, filteredOptions, value]);

  return (
    <div className="timezone-select-field">
      <label htmlFor={id}>
        <span>{label}</span>
      </label>
      <div className="timezone-select-controls">
        <input
          type="search"
          className="timezone-search-input"
          placeholder="Filter timezone..."
          value={filter}
          disabled={disabled}
          onChange={(e) => setFilter(e.target.value)}
          aria-label="Filter timezone options"
          data-testid="timezone-search-input"
        />
        <select
          id={id}
          className="timezone-select"
          value={value}
          disabled={disabled}
          onChange={(e) => onChange(e.target.value)}
          data-testid="timezone-select"
        >
          {optionsToRender.map((option: TimezoneOption) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
      {help ? <p className="settings-help">{help}</p> : null}
    </div>
  );
}
