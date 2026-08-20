# i18n Rule

When editing React components or adding new UI text, always use `useTranslation` from `react-i18next` and add the corresponding keys to both `web/src/i18n/locales/en/setup.json` and `web/src/i18n/locales/pt-BR/setup.json`.
Do NOT hardcode strings in English or Portuguese directly in `.tsx` or `.ts` files, unless they are internal logic/constants not displayed to the user.

Ensure to verify that newly added translation keys are complete and not accidentally copied as English into the `pt-BR` json file.
