import js from '@eslint/js'
import stylistic from '@stylistic/eslint-plugin'
import eslintConfigPrettier from 'eslint-config-prettier/flat'
import react from 'eslint-plugin-react'
import reactHooks from 'eslint-plugin-react-hooks'
import simpleImportSort from 'eslint-plugin-simple-import-sort'
import globals from 'globals'
import tseslint from 'typescript-eslint'

const sourceFiles = ['**/*.{js,jsx,ts,tsx}']

export default tseslint.config(
    {
        ignores: [
            '**/node_modules/**',
            'dist/**',
            'dist-ssr/**',
            'target/**',
            'test_export/**',
            'src/bindings.ts',
        ],
    },
    js.configs.recommended,
    ...tseslint.configs.recommended,
    {
        ...react.configs.flat.recommended,
        files: sourceFiles,
    },
    {
        ...react.configs.flat['jsx-runtime'],
        files: sourceFiles,
    },
    stylistic.configs.customize({
        indent: 4,
        quotes: 'single',
        semi: false,
        jsx: true,
        commaDangle: 'always-multiline',
        arrowParens: true,
    }),
    {
        files: sourceFiles,
        plugins: {
            'react-hooks': reactHooks,
            'simple-import-sort': simpleImportSort,
        },
        languageOptions: {
            ecmaVersion: 'latest',
            sourceType: 'module',
            globals: {
                ...globals.browser,
                ...globals.node,
            },
            parserOptions: {
                ecmaFeatures: {
                    jsx: true,
                },
            },
        },
        settings: {
            react: {
                version: 'detect',
            },
        },
        rules: {
            '@stylistic/eol-last': ['error', 'always'],
            '@stylistic/indent': [
                'error',
                4,
                {
                    ImportDeclaration: 1,
                    SwitchCase: 1,
                },
            ],
            '@stylistic/jsx-quotes': ['error', 'prefer-single'],
            '@stylistic/linebreak-style': ['error', 'unix'],
            '@stylistic/no-multi-spaces': ['error'],
            '@stylistic/no-trailing-spaces': 'error',
            '@stylistic/object-curly-newline': [
                'error',
                {
                    ImportDeclaration: {
                        multiline: true,
                        minProperties: 2,
                    },
                    ExportDeclaration: {
                        multiline: true,
                        minProperties: 2,
                    },
                },
            ],
            '@stylistic/padding-line-between-statements': [
                'error',
                {
                    blankLine: 'always',
                    prev: 'import',
                    next: '*',
                },
                {
                    blankLine: 'any',
                    prev: 'import',
                    next: 'import',
                },
            ],
            '@stylistic/quotes': [
                'error',
                'single',
                {
                    avoidEscape: true,
                },
            ],
            '@stylistic/rest-spread-spacing': ['error', 'never'],
            '@stylistic/semi': ['error', 'never'],
            '@stylistic/template-curly-spacing': ['error', 'never'],
            '@typescript-eslint/consistent-type-imports': [
                'error',
                {
                    disallowTypeAnnotations: false,
                },
            ],
            '@typescript-eslint/no-import-type-side-effects': 'error',
            '@typescript-eslint/no-unused-vars': ['error'],
            'no-duplicate-imports': ['error'],
            'no-multi-spaces': 'off',
            'no-prototype-builtins': 'off',
            'no-trailing-spaces': 'off',
            'no-unneeded-ternary': [
                'error',
                {
                    defaultAssignment: false,
                },
            ],
            'no-unused-vars': 'off',
            'no-useless-computed-key': 'error',
            'no-useless-constructor': 'error',
            'no-var': 'error',
            'prefer-const': [
                'error',
                {
                    destructuring: 'any',
                    ignoreReadBeforeAssign: true,
                },
            ],
            'prefer-destructuring': [
                'error',
                {
                    VariableDeclarator: {
                        array: false,
                        object: true,
                    },
                    AssignmentExpression: {
                        array: true,
                        object: false,
                    },
                },
                {
                    enforceForRenamedProperties: false,
                },
            ],
            'prefer-numeric-literals': 'error',
            'prefer-rest-params': 'error',
            'prefer-spread': 'error',
            'prefer-template': 'error',
            'react/jsx-boolean-value': [
                'error',
                'never',
                {
                    always: [],
                },
            ],
            'react/jsx-closing-bracket-location': ['error', 'line-aligned'],
            'react/jsx-closing-tag-location': 'error',
            'react/jsx-curly-spacing': [
                'error',
                'never',
                {
                    allowMultiline: true,
                },
            ],
            'react/jsx-handler-names': [
                'off',
                {
                    eventHandlerPrefix: 'handle',
                    eventHandlerPropPrefix: 'on',
                },
            ],
            'react/jsx-indent-props': ['error', 4],
            'react/jsx-max-props-per-line': [
                'error',
                {
                    maximum: 1,
                    when: 'multiline',
                },
            ],
            'react/jsx-no-duplicate-props': [
                'error',
                {
                    ignoreCase: true,
                },
            ],
            'react/jsx-no-undef': 'error',
            'react/jsx-pascal-case': [
                'error',
                {
                    allowAllCaps: true,
                    ignore: [],
                },
            ],
            'react/jsx-uses-vars': 'error',
            'react/prop-types': 'off',
            'react-hooks/exhaustive-deps': 'warn',
            'react-hooks/rules-of-hooks': 'error',
            'require-yield': 'error',
            'simple-import-sort/exports': 'error',
            'simple-import-sort/imports': [
                'error',
                {
                    groups: [
                        ['^react$', '^@?\\w'],
                        ['^@/'],
                        ['^\\.\\.(?!/?$)', '^\\.\\./?$'],
                        ['^\\./(?=.*/)(?!/?$)', '^\\.(?!/?$)', '^\\./?$'],
                        ['^\\u0000'],
                    ],
                },
            ],
        },
    },
    eslintConfigPrettier,
)
