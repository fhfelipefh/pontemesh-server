import { describe, expect, it } from "vitest";
import { checkLocaleConsistency, flattenJson, pruneJsonObject, pruneLocaleFile, scanSourceFiles } from "./prune-i18n";

describe("prune-i18n", () => {
  it("detects direct t calls and Trans keys", () => {
    const result = scanSourceFiles([
      {
        path: "src/page.tsx",
        relativePath: "src/page.tsx",
        content: "const { t } = useTranslation(); t(\"setup.title\"); <Trans i18nKey=\"setup.description\" />;"
      }
    ]);

    expect([...result.usedKeys].sort()).toEqual(["setup.description", "setup.title"]);
  });

  it("detects namespace-qualified t calls", () => {
    const result = scanSourceFiles([
      {
        path: "src/page.tsx",
        relativePath: "src/page.tsx",
        content: "const { t } = useTranslation(\"settings\"); t(\"title\"); t(\"save\", { ns: \"common\" });"
      }
    ]);

    expect([...result.usedKeys].sort()).toEqual(["common.save", "settings.title"]);
  });

  it("removes unused keys without breaking JSON shape", () => {
    const pruned = pruneJsonObject(
      {
        setup: {
          title: "Title",
          unused: "Unused",
          emptyParent: {
            child: "Unused child"
          }
        }
      },
      (key) => key === "setup.title"
    );

    expect(pruned.value).toEqual({ setup: { title: "Title" } });
    expect(pruned.removedKeys.sort()).toEqual(["setup.emptyParent.child", "setup.unused"]);
    expect(flattenJson(pruned.value)).toEqual(["setup.title"]);
  });

  it("preserves allowlisted keys and prefixes", () => {
    const result = pruneLocaleFile(
      {
        language: "en",
        namespace: "setup",
        path: "setup.json",
        relativePath: "en/setup.json",
        content: {
          setup: {
            title: "Title",
            status: {
              active: "Active"
            },
            errors: {
              generic: "Error"
            },
            unused: "Unused"
          }
        }
      },
      new Set(["setup.title"]),
      {
        keys: ["setup.status.active"],
        prefixes: ["setup.errors."]
      }
    );

    expect(result.content).toEqual({
      setup: {
        title: "Title",
        status: { active: "Active" },
        errors: { generic: "Error" }
      }
    });
    expect(result.protectedKeys.sort()).toEqual(["setup.errors.generic", "setup.status.active"]);
    expect(result.removedKeys).toEqual(["setup.unused"]);
  });

  it("reports dynamic usages", () => {
    const result = scanSourceFiles([
      {
        path: "src/page.tsx",
        relativePath: "src/page.tsx",
        content: "t(`status.${status}`);\nt(key);"
      }
    ]);

    expect(result.dynamicUsages).toEqual([
      { file: "src/page.tsx", line: 1, expression: "t(`status.${status}`)" },
      { file: "src/page.tsx", line: 2, expression: "t(key)" }
    ]);
  });

  it("fails consistency when languages differ", () => {
    const errors = checkLocaleConsistency([
      {
        language: "en",
        namespace: "setup",
        path: "en/setup.json",
        relativePath: "en/setup.json",
        content: { setup: { title: "Title" } }
      },
      {
        language: "pt-BR",
        namespace: "setup",
        path: "pt-BR/setup.json",
        relativePath: "pt-BR/setup.json",
        content: { setup: { title: "Titulo", description: "Descricao" } }
      }
    ]);

    expect(errors).toEqual(["en is missing setup.description"]);
  });
});
