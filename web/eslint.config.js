import importPlugin from "eslint-plugin-import";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";

export default [
  {
    ignores: ["dist", "node_modules"]
  },
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        project: ["./tsconfig.json", "./tsconfig.node.json"],
        noWarnOnMultipleProjects: true,
        tsconfigRootDir: import.meta.dirname,
        ecmaVersion: "latest",
        sourceType: "module"
      }
    },
    plugins: {
      "@typescript-eslint": tseslint,
      import: importPlugin,
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh
    },
    settings: {
      "import/resolver": {
        typescript: {
          project: ["./tsconfig.json", "./tsconfig.node.json"],
          noWarnOnMultipleProjects: true
        },
        node: {
          extensions: [".js", ".jsx", ".ts", ".tsx"]
        }
      }
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      ...reactHooks.configs.recommended.rules,
      "import/no-unresolved": "error",
      "react-refresh/only-export-components": [
        "warn",
        { allowConstantExport: true }
      ]
    }
  }
];
