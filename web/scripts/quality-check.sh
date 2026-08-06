#!/bin/bash

# Run all quality checks
echo "Running TypeScript type checking..."
npm run typecheck

echo "Running ESLint..."
npm run lint

echo "Running UI quality tests..."
npm run test:ui:quality

echo "Running accessibility tests..."
npm run test:ui:quality tests/ui-quality/accessibility.test.ts

echo "All quality checks completed!"
