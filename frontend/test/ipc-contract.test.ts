import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import ts from 'typescript';

test('real Rust Serde fixtures satisfy the frontend IPC types', () => {
  const fixture = readFileSync(new URL('./fixtures/rust-ipc.json', import.meta.url), 'utf8').trim();
  const filename = fileURLToPath(new URL('./rust-ipc-typecheck.ts', import.meta.url)).replace(
    /\\/g,
    '/',
  );
  const source = `
    import type { Reply, FinishRequest, IpcRequest, ErrorCode } from '../src/models.ts';
    type DeepReadonly<T> = { readonly [K in keyof T]: DeepReadonly<T[K]> };
    const fixture = ${fixture} as const;
    const reply: DeepReadonly<Reply> = fixture.reply;
    const failure: DeepReadonly<Reply> = fixture.failureReply;
    const finish: DeepReadonly<IpcRequest<FinishRequest>> = fixture.finishRequest;
    const defaults: DeepReadonly<IpcRequest<FinishRequest>> = fixture.defaultFinishRequest;
    const errors: readonly { readonly code: ErrorCode; readonly message: string }[] = fixture.errors;
  `;
  const options: ts.CompilerOptions = {
    strict: true,
    noEmit: true,
    skipLibCheck: true,
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Bundler,
    allowImportingTsExtensions: true,
    types: [],
  };
  function typecheck(text: string) {
    const host = ts.createCompilerHost(options);
    const getSourceFile = host.getSourceFile.bind(host);
    host.getSourceFile = (path, version, onError, shouldCreateNewSourceFile) =>
      path.replace(/\\/g, '/') === filename
        ? ts.createSourceFile(path, text, version, true)
        : getSourceFile(path, version, onError, shouldCreateNewSourceFile);
    return ts.getPreEmitDiagnostics(ts.createProgram([filename], options, host));
  }
  const diagnostics = typecheck(source);
  assert.equal(
    diagnostics.length,
    0,
    ts.formatDiagnosticsWithColorAndContext(diagnostics, {
      getCanonicalFileName: (path) => path,
      getCurrentDirectory: () => process.cwd(),
      getNewLine: () => '\n',
    }),
  );
  const invalid = JSON.parse(fixture);
  invalid.finishRequest.operation.command.activityId = 42;
  const invalidSource = source.replace(fixture, JSON.stringify(invalid));
  assert.notEqual(invalidSource, source);
  assert.ok(typecheck(invalidSource).some((diagnostic) => diagnostic.code === 2322));
});
