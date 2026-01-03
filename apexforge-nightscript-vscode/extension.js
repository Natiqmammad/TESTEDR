const vscode = require('vscode');
const { execFile } = require('child_process');
const path = require('path');

const KEYWORDS = [
  'fun', 'async', 'await', 'let', 'var', 'return', 'if', 'else', 'while', 'for', 'switch',
  'try', 'catch', 'struct', 'enum', 'trait', 'impl', 'import', 'as', 'extern', 'unsafe', 'assembly',
  'slice', 'tuple', 'mut', 'widget', 'state', 'build', 'setState', 'apex',
  'android', 'react', 'web', 'math', 'forge', 'log', 'result', 'option', 'vec', 'str', 'map', 'set',
  'MethodCall', 'MethodResponse', 'MethodError', 'BuildContext', 'RenderObject',
  'FutureHandle', 'FutureState', 'FutureKind', 'Executor', 'TaskGroup',

  // async
  'async.sleep', 'async.timeout', 'async.spawn', 'async.parallel', 'async.race', 'async.all',
  'async.any', 'async.then', 'async.catch', 'async.finally', 'async.cancel', 'async.is_cancelled',
  'async.channel', 'async.stream', 'async.yield', 'async.interval', 'async.retry',

  // widgets
  'ctx.text', 'ctx.button', 'ctx.column', 'ctx.row', 'ctx.container', 'ctx.image', 'ctx.list',
  'ctx.appbar', 'ctx.scaffold', 'ctx.widget',

  // android
  'android.permissions', 'android.intent', 'android.storage',

  // forge.fs
  'fs.read_to_string', 'fs.read_bytes', 'fs.write_string', 'fs.write_bytes', 'fs.append_string',
  'fs.append_bytes', 'fs.create_dir', 'fs.create_dir_all', 'fs.remove_dir', 'fs.remove_dir_all',
  'fs.copy_file', 'fs.move', 'fs.remove_file', 'fs.ensure_dir', 'fs.read_lines', 'fs.write_lines',
  'fs.copy_dir_recursive', 'fs.join', 'fs.dirname', 'fs.basename', 'fs.extension', 'fs.canonicalize',
  'fs.metadata', 'fs.exists', 'fs.is_file', 'fs.is_dir', 'fs.read_dir',

  // forge.net
  'net.tcp_connect', 'net.tcp_listen', 'net.tcp_accept', 'net.tcp_send', 'net.tcp_recv',
  'net.tcp_shutdown', 'net.tcp_set_nodelay', 'net.tcp_set_read_timeout', 'net.tcp_set_write_timeout',
  'net.tcp_peer_addr', 'net.tcp_local_addr', 'net.udp_bind', 'net.udp_connect', 'net.udp_send',
  'net.udp_send_to', 'net.udp_recv', 'net.udp_recv_from', 'net.udp_set_broadcast',
  'net.udp_set_read_timeout', 'net.udp_set_write_timeout', 'net.udp_peer_addr', 'net.udp_local_addr',
  'net.close_socket', 'net.close_listener',

  // forge.db
  'db.open', 'db.exec', 'db.query', 'db.begin', 'db.commit', 'db.rollback',
  'db.get', 'db.set', 'db.del', 'db.close'
];

let diagnosticCollection;
let outputChannel;

function activate(context) {
  const provider = vscode.languages.registerCompletionItemProvider(
    { language: 'afml' },
    {
      provideCompletionItems() {
        return KEYWORDS.map((label) => {
          const item = new vscode.CompletionItem(label);
          item.kind = vscode.CompletionItemKind.Keyword;
          item.insertText = label;
          return item;
        });
      }
    },
    '.', ':' // trigger characters
  );

  diagnosticCollection = vscode.languages.createDiagnosticCollection('afml');
  outputChannel = vscode.window.createOutputChannel('ApexForge apexrc');

  const runCheckCommand = vscode.commands.registerCommand(
    'apexforge.runApexrcCheck',
    () => runApexrcCheck()
  );

  context.subscriptions.push(provider, diagnosticCollection, outputChannel, runCheckCommand);

  // Auto-run checks on save and open
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      if (doc.languageId === 'afml') runApexrcCheck(doc);
    }),
    vscode.workspace.onDidOpenTextDocument((doc) => {
      if (doc.languageId === 'afml') runApexrcCheck(doc);
    })
  );

  // Run once on workspace load
  if (vscode.workspace.workspaceFolders?.length > 0) {
    runApexrcCheck();
  }
}

function deactivate() {
  diagnosticCollection?.dispose();
  outputChannel?.dispose();
}

function getWorkspaceFolder(document) {
  if (document) {
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (folder) return folder;
  }
  return vscode.workspace.workspaceFolders?.[0];
}

function runApexrcCheck(document) {
  const folder = getWorkspaceFolder(document);
  if (!folder) return;

  const cfg = vscode.workspace.getConfiguration('apexforge');
  const apexrcPath = cfg.get('apexrcPath', 'apexrc');
  const args = cfg.get('apexrcCheckArgs', ['check']);

  outputChannel.appendLine(`[apexrc] running ${apexrcPath} ${args.join(' ')} in ${folder.uri.fsPath}`);

  execFile(apexrcPath, args, { cwd: folder.uri.fsPath }, (error, stdout, stderr) => {
    const stdOutput = stdout?.toString() ?? '';
    const errOutput = stderr?.toString() ?? '';
    
    outputChannel.appendLine(`[DEBUG] apexrcPath: ${apexrcPath}`);
    outputChannel.appendLine(`[DEBUG] args: ${args.join(' ')}`);
    outputChannel.appendLine(`[DEBUG] cwd: ${folder.uri.fsPath}`);
    outputChannel.appendLine(`[DEBUG] error: ${error}`);
    outputChannel.appendLine(`[DEBUG] stdout: ${stdOutput}`);
    outputChannel.appendLine(`[DEBUG] stderr: ${errOutput}`);

    if (errOutput.trim()) {
      outputChannel.appendLine(errOutput.trim());
    }

    if (error) {
      outputChannel.appendLine(`[apexrc] exited with code ${error.code ?? 'unknown'}`);

      const parsed = parseDiagnostic(stdOutput || errOutput, folder.uri.fsPath);
      if (parsed) {
        diagnosticCollection.set(parsed.uri, [parsed.diagnostic]);
      } else {
        diagnosticCollection.clear();
        vscode.window.showErrorMessage('apexrc check failed (see output).');
      }
      return;
    }

    diagnosticCollection.clear();
    if (stdOutput.trim()) {
      outputChannel.appendLine(stdOutput.trim());
    } else {
      outputChannel.appendLine('[apexrc] check succeeded');
    }
  });
}

function parseDiagnostic(raw, workspacePath) {
  if (!raw.trim()) return;

  const lines = raw.split(/\r?\n/).filter((l) => l.trim());
  if (!lines.length) return;

  const fileLine = lines[0].replace(/:$/, '').trim();
  const errorLine = lines.find((l) => l.startsWith('error:'));
  if (!errorLine) return;

  const message = errorLine.replace(/^error:\s*/, '').trim();

  const locationLine = lines.find((l) => l.includes(' --> line '));
  let lineNumber = 0;
  let columnNumber = 0;

  if (locationLine) {
    const match = locationLine.match(/line (\d+), column (\d+)/);
    if (match) {
      lineNumber = parseInt(match[1], 10) - 1;
      columnNumber = parseInt(match[2], 10) - 1;
    }
  }

  const filePath = path.isAbsolute(fileLine)
    ? fileLine
    : path.join(workspacePath, fileLine);

  const uri = vscode.Uri.file(filePath);

  const range = new vscode.Range(
    new vscode.Position(lineNumber, columnNumber),
    new vscode.Position(lineNumber, columnNumber + 1)
  );

  const diagnostic = new vscode.Diagnostic(range, message, vscode.DiagnosticSeverity.Error);
  diagnostic.source = 'apexrc';

  return { uri, diagnostic };
}

module.exports = { activate, deactivate };
