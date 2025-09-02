// ESLint configuration for lite-wallet crypto project
// Security-focused linting rules for cryptocurrency wallet development
// Enforces patterns defined in Cursor Rules for safe crypto development

import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import svelteConfig from '@sveltejs/eslint-config';

export default [
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  ...svelteConfig,

  {
    languageOptions: {
      globals: {
        // Tauri globals
        __TAURI__: 'readonly',
        __TAURI_METADATA__: 'readonly',
      },
    },
  },

  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
      },
    },
  },

  {
    // Custom rules for crypto wallet security
    rules: {
      // Security: Prevent console.log with sensitive data
      'no-console': [
        'warn',
        {
          allow: ['warn', 'error', 'info'],
        },
      ],

      // Security: Require explicit any types (avoid accidental sensitive data exposure)
      '@typescript-eslint/no-explicit-any': 'error',

      // Security: Prevent unused variables (could contain sensitive data)
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],

      // Code quality: Prefer const for immutability
      'prefer-const': 'error',

      // Security: No eval (code injection prevention)
      'no-eval': 'error',
      'no-implied-eval': 'error',

      // Security: Prevent prototype pollution
      'no-prototype-builtins': 'error',

      // Code quality: Consistent return values
      'consistent-return': 'error',

      // Security: Require proper error handling
      'no-empty-catch': 'error',

      // Svelte-specific rules
      'svelte/no-unused-svelte-ignore': 'error',
      'svelte/no-useless-mustaches': 'warn',
      'svelte/prefer-destructuring-props': 'warn',

      // TypeScript-specific security rules
      '@typescript-eslint/ban-ts-comment': [
        'error',
        {
          'ts-expect-error': 'allow-with-description',
        },
      ],
      '@typescript-eslint/no-non-null-assertion': 'error',

      // Import organization (matches our naming conventions)
      'sort-imports': [
        'warn',
        {
          ignoreCase: true,
          ignoreDeclarationSort: true,
        },
      ],
    },
  },

  {
    // Special rules for sensitive files
    files: ['**/machines/**', '**/stores/**', '**/utils/crypto*'],
    rules: {
      // Extra strict for state management and crypto files
      'no-console': ['error', { allow: ['error'] }],
      '@typescript-eslint/explicit-function-return-type': 'warn',
      'no-var': 'error',
      'prefer-const': 'error',
    },
  },

  {
    // Tauri command files - extra security
    files: ['src-tauri/**/*.rs'],
    rules: {
      // Note: ESLint can't parse Rust, but keeping for future JS command wrappers
    },
  },

  {
    // Test files - more relaxed rules
    files: ['**/*.test.{js,ts,svelte}', '**/*.spec.{js,ts,svelte}'],
    rules: {
      '@typescript-eslint/no-explicit-any': 'off',
      'no-console': 'off',
    },
  },

  // Ignore patterns
  {
    ignores: [
      'build/',
      '.svelte-kit/',
      'dist/',
      'node_modules/',
      'src-tauri/target/',
      'src-tauri/gen/',
      '*.env*',
      'static/',
      'playwright-report/',
      'test-results/',
    ],
  },
];
