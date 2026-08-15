export type TimezoneOption = {
  value: string;
  label: string;
  city: string;
  offset: string;
  searchKey: string;
};

const COMMON_TIMEZONES = [
  "UTC",
  "America/Sao_Paulo",
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "America/Anchorage",
  "America/Adak",
  "Honolulu",
  "America/Argentina/Buenos_Aires",
  "America/Bogota",
  "America/Caracas",
  "America/Santiago",
  "America/Lima",
  "America/Mexico_City",
  "America/Toronto",
  "America/Vancouver",
  "Europe/London",
  "Europe/Berlin",
  "Europe/Paris",
  "Europe/Madrid",
  "Europe/Rome",
  "Europe/Amsterdam",
  "Europe/Brussels",
  "Europe/Lisbon",
  "Europe/Moscow",
  "Europe/Athens",
  "Europe/Zurich",
  "Asia/Tokyo",
  "Asia/Shanghai",
  "Asia/Hong_Kong",
  "Asia/Singapore",
  "Asia/Dubai",
  "Asia/Kolkata",
  "Asia/Bangkok",
  "Asia/Seoul",
  "Australia/Sydney",
  "Australia/Melbourne",
  "Australia/Brisbane",
  "Australia/Perth",
  "Pacific/Auckland",
  "Africa/Johannesburg",
  "Africa/Cairo",
  "Africa/Lagos"
];

const formatterCache = new Map<string, Intl.DateTimeFormat>();

export function getTimezoneOffsetString(timeZone: string, date = new Date()): string {
  try {
    let formatter = formatterCache.get(timeZone);
    if (!formatter) {
      formatter = new Intl.DateTimeFormat("en-US", {
        timeZone,
        timeZoneName: "longOffset"
      });
      formatterCache.set(timeZone, formatter);
    }
    const parts = formatter.formatToParts(date);
    const tzPart = parts.find((p) => p.type === "timeZoneName");
    if (tzPart && tzPart.value) {
      let val = tzPart.value.replace("GMT", "UTC");
      if (val === "UTC") {
        val = "UTC+00:00";
      }
      return val;
    }
  } catch {
  }
  return "UTC+00:00";
}

export function formatTimezoneCity(timeZone: string): string {
  if (timeZone === "UTC") {
    return "UTC";
  }
  const parts = timeZone.split("/");
  const lastPart = parts[parts.length - 1];
  return lastPart.replace(/_/g, " ");
}

export function getAllTimezones(): string[] {
  if (typeof Intl !== "undefined" && typeof (Intl as unknown as { supportedValuesOf?: (key: string) => string[] }).supportedValuesOf === "function") {
    try {
      const supported = (Intl as unknown as { supportedValuesOf: (key: string) => string[] }).supportedValuesOf("timeZone");
      if (Array.isArray(supported) && supported.length > 0) {
        return supported;
      }
    } catch {
    }
  }
  return COMMON_TIMEZONES;
}

export function getTimezoneOptions(date = new Date()): TimezoneOption[] {
  const zoneList = getAllTimezones();
  const optionsMap = new Map<string, TimezoneOption>();

  const allZones = Array.from(new Set(["UTC", ...COMMON_TIMEZONES, ...zoneList]));

  for (const timeZone of allZones) {
    const city = formatTimezoneCity(timeZone);
    const offset = getTimezoneOffsetString(timeZone, date);
    const label = `${city} (${offset})`;
    const searchKey = `${city} ${timeZone} ${offset} ${label}`.toLowerCase();
    optionsMap.set(timeZone, {
      value: timeZone,
      label,
      city,
      offset,
      searchKey
    });
  }

  return Array.from(optionsMap.values());
}
