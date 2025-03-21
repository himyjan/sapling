/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type {Plugin, PluginOption} from 'vite';

import react from '@vitejs/plugin-react';
import fs, {existsSync} from 'node:fs';
import path from 'node:path';
import {defineConfig} from 'vite';
import styleX from 'vite-plugin-stylex';
import viteTsconfigPaths from 'vite-tsconfig-paths';

// Normalize `c:\foo\index.html` to `c:/foo/index.html`.
// This affects Rollup's `facadeModuleId` (which expects the `c:/foo/bar` format),
// and is important for Vite to replace the script tags in HTML files.
// See https://github.com/vitejs/vite/blob/7440191715b07a50992fcf8c90d07600dffc375e/packages/vite/src/node/plugins/html.ts#L804
// Without this, building on Windows might produce HTML entry points with
// missing `<script>` tags, resulting in a blank page.
function normalizeInputPath(inputPath: string) {
  return process.platform === 'win32' ? path.resolve(inputPath).replace(/\\/g, '/') : inputPath;
}

const isInternal = existsSync(path.resolve(__dirname, 'facebook/README.facebook.md'));

const input = [normalizeInputPath('webview.html')];
if (isInternal) {
  // Currently, the inline comment webview is not used in OSS
  input.push(normalizeInputPath('inlineCommentWebview.html'));
  input.push(normalizeInputPath('DiffCommentPanelWebview.html'));
}

console.log(isInternal ? 'Building internal version' : 'Building OSS version');

// vite-plugin-stylex doesn't support renaming the output CSS file, so we have to do that ourselves.
function moveStylexFilenamePlugin(): Plugin {
  return {
    name: 'move-stylex-filename',
    writeBundle(options, bundle) {
      for (const name in bundle) {
        const chunk = bundle[name];
        // Check if this is the stylex output cssfile
        if (chunk.type === 'asset' && /assets[/\\]stylex\.[a-f0-9]+\.css/.test(chunk.fileName)) {
          // Rename the file, move it from "assets" to "res" where the rest of our assets are
          const newName = 'res/stylex.css';
          if (options.dir == null) {
            this.error('Could not replace StyleX output, dir must be set');
          }
          const dir = options.dir as string;
          const oldPath = path.resolve(dir, chunk.fileName);
          const newPath = path.resolve(dir, newName);
          this.info(`Replacing StyleX output file ${chunk.fileName} with ${newName}`);
          fs.renameSync(oldPath, newPath);
          // Update the bundle object
          chunk.fileName = newName;
          bundle[newName] = chunk;
          delete bundle[name];
        }
      }
    },
  };
}

const replaceFiles = (
  replacements?: Array<{
    file: string;
    replacement: string;
  }>,
): PluginOption => {
  const projectRoot = process.cwd();
  replacements = replacements?.map(x => ({
    file: path.join(projectRoot, x.file),
    replacement: path.join(projectRoot, x.replacement),
  }));

  return {
    name: 'vite-plugin-replace-files',
    enforce: 'pre',
    async resolveId(source: string, importer: string | undefined, options: any) {
      const resolvedFile = await this.resolve(source, importer, {
        ...options,
        ...{skipSelf: true},
      });

      const foundReplacementFile = replacements?.find(
        replacement => replacement.file == resolvedFile?.id,
      );

      if (foundReplacementFile) {
        return {
          id: foundReplacementFile.replacement,
        };
      }
      return null;
    },
  };
};

export default defineConfig(({mode}) => ({
  base: '',
  plugins: [
    replaceFiles([
      {
        file: '../isl/src/platform.ts',
        replacement: './webview/vscodeWebviewPlatform.tsx',
      },
    ]),
    react({
      babel: {
        plugins: [
          [
            'jotai/babel/plugin-debug-label',
            {
              customAtomNames: [
                'atomFamilyWeak',
                'atomLoadableWithRefresh',
                'atomWithOnChange',
                'atomWithRefresh',
                'configBackedAtom',
                'jotaiAtom',
                'lazyAtom',
                'localStorageBackedAtom',
              ],
            },
          ],
        ],
      },
    }),
    styleX(),
    viteTsconfigPaths(),
    moveStylexFilenamePlugin(),
  ],
  build: {
    outDir: 'dist/webview',
    manifest: true,
    // FIXME: This means that all webviews will use the same css file.
    // This is too bloated for the inline comment webview and marginally slows down startup time.
    // Ideally, we'd load all the relevant css files in the webview, but our current approach
    // with our own manual copy of html in htmlForWebview does not support this.
    cssCodeSplit: false,
    rollupOptions: {
      input,
      output: {
        // Don't use hashed names, so ISL webview panel can pre-define what filename to load
        entryFileNames: '[name].js',
        chunkFileNames: '[name].js',
        assetFileNames: 'res/[name].[ext]',
      },
    },
    copyPublicDir: true,
    // No need for source maps in production. We can always build them locally to understand a wild stack trace.
    sourcemap: mode === 'development',
  },
  worker: {
    rollupOptions: {
      output: {
        // Don't use hashed names, so ISL webview panel can pre-define what filename to load
        entryFileNames: 'worker/[name].js',
        chunkFileNames: 'worker/[name].js',
        assetFileNames: 'worker/[name].[ext]',
      },
    },
  },
  publicDir: '../isl/public',
  server: {
    // No need to open the browser, we run inside vscode and don't really connect to the server.
    open: false,
    port: 3005,
    cors: {
      origin: /^vscode-webview:\/\/.*/,
    },
  },
}));
