import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

type JsonValue = string | number | boolean | null | JsonObject | JsonValue[];
type JsonObject = { [key: string]: JsonValue };

type LocaleFile = {
  language: string;
  namespace: string;
  path: string;
  relativePath: string;
  content: JsonObject;
};

type Allowlist = {
  keys: string[];
  prefixes: string[];
};

type SourceFile = {
  path: string;
  relativePath: string;
  content: string;
};

type DynamicUsage = {
  file: string;
  line: number;
  expression: string;
};

type UsageScanResult = {
  usedKeys: Set<string>;
  dynamicUsages: DynamicUsage[];
};

type PruneResult = {
  content: JsonObject;
  removedKeys: string[];
  protectedKeys: string[];
};

type RunResult = {
  usedKeys: string[];
  removedKeys: string[];
  protectedKeys: string[];
  dynamicUsages: DynamicUsage[];
  consistencyErrors: string[];
  wouldChange: boolean;
};

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const webRoot = path.resolve(__dirname, "..");
const defaultSourceRoot = path.join(webRoot, "src");
const defaultLocalesRoot = path.join(defaultSourceRoot, "i18n", "locales");
const defaultAllowlistPath = path.join(__dirname, "i18n-allowlist.json");

const stringLiteral = "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|`([^`$\\\\]*(?:\\\\.[^`$\\\\]*)*)`";

export function flattenJson(value: JsonValue, prefix = ""): string[] {
  if (!isPlainObject(value)) {
    return prefix ? [prefix] : [];
  }

  return Object.keys(value).flatMap((key) => {
    const nextPrefix = prefix ? `${prefix}.${key}` : key;
    return flattenJson(value[key], nextPrefix);
  });
}

export function pruneJsonObject(
  value: JsonObject,
  shouldKeep: (key: string) => boolean,
  prefix = ""
): { value: JsonObject; removedKeys: string[] } {
  const nextValue: JsonObject = {};
  const removedKeys: string[] = [];

  for (const key of Object.keys(value)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    const item = value[key];

    if (isPlainObject(item)) {
      const pruned = pruneJsonObject(item, shouldKeep, fullKey);
      removedKeys.push(...pruned.removedKeys);

      if (Object.keys(pruned.value).length > 0) {
        nextValue[key] = pruned.value;
      }
      continue;
    }

    if (shouldKeep(fullKey)) {
      nextValue[key] = item;
    } else {
      removedKeys.push(fullKey);
    }
  }

  return { value: nextValue, removedKeys };
}

export function logicalKey(namespace: string, key: string): string {
  return key.startsWith(`${namespace}.`) ? key : `${namespace}.${key}`;
}

export function localKey(namespace: string, key: string): string {
  return key.startsWith(`${namespace}.`) ? key.slice(namespace.length + 1) : key;
}

export function scanSourceFiles(files: SourceFile[]): UsageScanResult {
  const usedKeys = new Set<string>();
  const dynamicUsages: DynamicUsage[] = [];

  for (const file of files) {
    const defaultNamespaces = findDefaultNamespaces(file.content);

    for (const match of file.content.matchAll(new RegExp(`(?:^|[^\\w.])(?:i18n\\.)?t\\(\\s*(${stringLiteral})([\\s\\S]*?)\\)`, "g"))) {
      const key = unescapeLiteral(match[1]);
      const args = match[5] ?? "";
      const ns = findOptionNamespace(args);
      addUsedKey(usedKeys, key, ns ? [ns] : defaultNamespaces);
    }

    for (const match of file.content.matchAll(/<Trans\b[^>]*\bi18nKey\s*=\s*(?:"([^"]+)"|'([^']+)')/g)) {
      const key = match[1] ?? match[2];
      addUsedKey(usedKeys, key, undefined);
    }

    collectDynamicUsages(file, dynamicUsages);
  }

  return { usedKeys, dynamicUsages };
}

export function pruneLocaleFile(file: LocaleFile, usedKeys: Set<string>, allowlist: Allowlist): PruneResult {
  const protectedKeys: string[] = [];
  const allowed = new Set(allowlist.keys);
  const prefixes = allowlist.prefixes;

  const shouldKeep = (key: string) => {
    const fullKey = logicalKey(file.namespace, key);
    const protectedByAllowlist = allowed.has(fullKey) || prefixes.some((prefix) => fullKey.startsWith(prefix));

    if (protectedByAllowlist) {
      protectedKeys.push(fullKey);
      return true;
    }

    return usedKeys.has(fullKey);
  };

  const pruned = pruneJsonObject(file.content, shouldKeep);

  return {
    content: pruned.value,
    removedKeys: pruned.removedKeys.map((key) => logicalKey(file.namespace, key)),
    protectedKeys: uniqueSorted(protectedKeys)
  };
}

export function checkLocaleConsistency(files: LocaleFile[]): string[] {
  const byLanguage = new Map<string, Set<string>>();

  for (const file of files) {
    const keys = byLanguage.get(file.language) ?? new Set<string>();
    for (const key of flattenJson(file.content)) {
      keys.add(logicalKey(file.namespace, key));
    }
    byLanguage.set(file.language, keys);
  }

  const allKeys = new Set<string>();
  for (const keys of byLanguage.values()) {
    for (const key of keys) {
      allKeys.add(key);
    }
  }

  const errors: string[] = [];
  for (const [language, keys] of byLanguage) {
    for (const key of allKeys) {
      if (!keys.has(key)) {
        errors.push(`${language} is missing ${key}`);
      }
    }
  }

  return errors.sort();
}

export async function runPrune(options: { write: boolean; sourceRoot?: string; localesRoot?: string; allowlistPath?: string }): Promise<RunResult> {
  const sourceRoot = options.sourceRoot ?? defaultSourceRoot;
  const localesRoot = options.localesRoot ?? defaultLocalesRoot;
  const allowlistPath = options.allowlistPath ?? defaultAllowlistPath;

  const [sourceFiles, localeFiles, allowlist] = await Promise.all([
    readSourceFiles(sourceRoot),
    readLocaleFiles(localesRoot),
    readAllowlist(allowlistPath)
  ]);

  const usage = scanSourceFiles(sourceFiles);
  const prunedFiles = localeFiles.map((file) => ({ file, result: pruneLocaleFile(file, usage.usedKeys, allowlist) }));
  const consistencyFiles = prunedFiles.map(({ file, result }) => ({ ...file, content: result.content }));
  const consistencyErrors = checkLocaleConsistency(consistencyFiles);

  const removedKeys = prunedFiles.flatMap(({ file, result }) => result.removedKeys.map((key) => `${file.language}/${key}`)).sort();
  const protectedKeys = uniqueSorted(prunedFiles.flatMap(({ result }) => result.protectedKeys));

  if (options.write && consistencyErrors.length === 0) {
    await Promise.all(prunedFiles.map(({ file, result }) => writeFile(file.path, `${JSON.stringify(result.content, null, 2)}\n`)));
  }

  return {
    usedKeys: [...usage.usedKeys].sort(),
    removedKeys,
    protectedKeys,
    dynamicUsages: usage.dynamicUsages,
    consistencyErrors,
    wouldChange: removedKeys.length > 0
  };
}

function addUsedKey(usedKeys: Set<string>, key: string, namespaces: string[] | undefined): void {
  if (key.includes(":")) {
    const [namespace, rest] = key.split(/:(.*)/s);
    usedKeys.add(logicalKey(namespace, rest));
    return;
  }

  if (key.includes(".")) {
    usedKeys.add(key);
    return;
  }

  for (const namespace of namespaces ?? ["translation"]) {
    usedKeys.add(logicalKey(namespace, key));
  }
}

function findDefaultNamespaces(content: string): string[] {
  const namespaces = new Set<string>();

  for (const match of content.matchAll(new RegExp(`useTranslation\\(\\s*(${stringLiteral})\\s*\\)`, "g"))) {
    namespaces.add(unescapeLiteral(match[1]));
  }

  for (const match of content.matchAll(/useTranslation\(\s*\[([\s\S]*?)\]\s*\)/g)) {
    for (const namespace of match[1].matchAll(new RegExp(stringLiteral, "g"))) {
      namespaces.add(unescapeLiteral(namespace[0]));
    }
  }

  return namespaces.size > 0 ? [...namespaces] : [];
}

function findOptionNamespace(args: string): string | undefined {
  const match = args.match(new RegExp(`\\bns\\s*:\\s*(${stringLiteral})`));
  return match ? unescapeLiteral(match[1]) : undefined;
}

function collectDynamicUsages(file: SourceFile, dynamicUsages: DynamicUsage[]): void {
  const seen = new Set<string>();
  const patterns = [
    /(?:^|[^\w.])(?:i18n\.)?t\(\s*([^"'`\s][^)]*?)\)/g,
    /(?:^|[^\w.])(?:i18n\.)?t\(\s*(`[^`]*\$\{[^`]*`|"[^"]*"\s*\+[^)]*|'[^']*'\s*\+[^)]*)\)/g
  ];

  for (const pattern of patterns) {
    for (const match of file.content.matchAll(pattern)) {
      const callIndex = (match.index ?? 0) + match[0].indexOf("t(");
      const expression = `t(${match[1].trim()})`;
      const key = `${file.relativePath}:${callIndex}:${expression}`;

      if (!seen.has(key)) {
        seen.add(key);
        dynamicUsages.push({
          file: file.relativePath,
          line: lineNumber(file.content, callIndex),
          expression
        });
      }
    }
  }

  dynamicUsages.sort((left, right) => left.file.localeCompare(right.file) || left.line - right.line || left.expression.localeCompare(right.expression));
}

function unescapeLiteral(value: string): string {
  const raw = value.slice(1, -1);
  return raw.replace(/\\(["'`\\])/g, "$1");
}

function lineNumber(content: string, index: number): number {
  return content.slice(0, index).split("\n").length;
}

function isPlainObject(value: JsonValue): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

async function readSourceFiles(root: string): Promise<SourceFile[]> {
  const paths = await listFiles(root, (filePath) => /\.(ts|tsx)$/.test(filePath) && !/\.test\.(ts|tsx)$/.test(filePath));
  return Promise.all(paths.map(async (filePath) => ({
    path: filePath,
    relativePath: path.relative(webRoot, filePath),
    content: await readFile(filePath, "utf8")
  })));
}

async function readLocaleFiles(root: string): Promise<LocaleFile[]> {
  const paths = await listFiles(root, (filePath) => filePath.endsWith(".json"));
  return Promise.all(paths.map(async (filePath) => {
    const relativePath = path.relative(root, filePath);
    const [language] = relativePath.split(path.sep);

    return {
      language,
      namespace: path.basename(filePath, ".json"),
      path: filePath,
      relativePath,
      content: JSON.parse(await readFile(filePath, "utf8")) as JsonObject
    };
  }));
}

async function readAllowlist(filePath: string): Promise<Allowlist> {
  const parsed = JSON.parse(await readFile(filePath, "utf8")) as Partial<Allowlist>;
  return {
    keys: parsed.keys ?? [],
    prefixes: parsed.prefixes ?? []
  };
}

async function listFiles(root: string, predicate: (filePath: string) => boolean): Promise<string[]> {
  const entries = await readdir(root, { withFileTypes: true });
  const files = await Promise.all(entries.map(async (entry) => {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      return listFiles(entryPath, predicate);
    }
    return predicate(entryPath) ? [entryPath] : [];
  }));

  return files.flat().sort();
}

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values)].sort();
}

function printReport(result: RunResult): void {
  console.log("i18n prune report\n");
  console.log(`Used keys: ${result.usedKeys.length}`);
  console.log(`Removed keys: ${result.removedKeys.length}`);
  console.log(`Protected keys: ${result.protectedKeys.length}`);
  console.log(`Dynamic usages detected: ${result.dynamicUsages.length}`);

  if (result.removedKeys.length === 0) {
    console.log("\nNo unused i18n keys found.");
  } else {
    console.log("\nRemoved:");
    for (const key of result.removedKeys) {
      console.log(`- ${key}`);
    }
  }

  if (result.protectedKeys.length > 0) {
    console.log("\nProtected:");
    for (const key of result.protectedKeys) {
      console.log(`- ${key}`);
    }
  }

  if (result.dynamicUsages.length > 0) {
    console.log("\nDynamic usages:");
    for (const usage of result.dynamicUsages) {
      console.log(`- ${usage.file}:${usage.line}: ${usage.expression}`);
    }
  }

  if (result.consistencyErrors.length > 0) {
    console.log("\nConsistency errors:");
    for (const error of result.consistencyErrors) {
      console.log(`- ${error}`);
    }
  }
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const write = args.includes("--write");
  const check = args.includes("--check");

  if (write === check) {
    console.error("Use exactly one mode: --write or --check.");
    process.exit(2);
  }

  const result = await runPrune({ write });
  printReport(result);

  if (result.consistencyErrors.length > 0) {
    process.exit(1);
  }

  if (check && result.wouldChange) {
    console.error("\nUnused i18n keys found. Run npm run i18n:prune to update locale files.");
    process.exit(1);
  }
}

if (process.argv[1] === __filename) {
  void main();
}
