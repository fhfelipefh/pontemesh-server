import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

type SourceFile = {
  path: string;
  relativePath: string;
  content: string;
};

const __filename = fileURLToPath(import.meta.url);
const srcRoot = path.dirname(__filename);
const themeTokensFile = path.join("styles", "theme.css");
const hexColorPattern = /#[0-9a-fA-F]{3,8}\b/g;
const colorFunctionPattern = /\b(?:rgb|rgba|hsl|hsla)\(([^)]*)\)/g;

describe("theme colors", () => {
  it("keeps fixed color values isolated in theme tokens", async () => {
    const files = await readSourceFiles(srcRoot);
    const violations = files.flatMap((file) => findFixedColorViolations(file));

    expect(violations).toEqual([]);
  });
});

function findFixedColorViolations(file: SourceFile): string[] {
  if (file.relativePath === themeTokensFile) {
    return [];
  }

  const violations: string[] = [];

  for (const match of file.content.matchAll(hexColorPattern)) {
    violations.push(`${file.relativePath}:${lineNumber(file.content, match.index ?? 0)} uses fixed color ${match[0]}`);
  }

  for (const match of file.content.matchAll(colorFunctionPattern)) {
    if (match[1].includes("var(")) {
      continue;
    }
    violations.push(`${file.relativePath}:${lineNumber(file.content, match.index ?? 0)} uses fixed color ${match[0]}`);
  }

  return violations;
}

async function readSourceFiles(root: string): Promise<SourceFile[]> {
  const paths = await listFiles(root, (filePath) => {
    const relativePath = path.relative(srcRoot, filePath);
    return (
      /\.(css|ts|tsx)$/.test(filePath) &&
      !relativePath.endsWith(".test.ts") &&
      !relativePath.endsWith(".test.tsx")
    );
  });

  return Promise.all(paths.map(async (filePath) => ({
    path: filePath,
    relativePath: path.relative(srcRoot, filePath),
    content: await readFile(filePath, "utf8")
  })));
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

function lineNumber(content: string, index: number): number {
  return content.slice(0, index).split("\n").length;
}
