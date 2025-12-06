const vscode = require('vscode');

const KEYWORDS = [
  'fun', 'async', 'await', 'let', 'var', 'return', 'if', 'else', 'while', 'for', 'switch',
  'try', 'catch', 'struct', 'enum', 'trait', 'impl', 'import', 'as', 'extern', 'unsafe', 'assembly',
  'slice', 'tuple', 'mut', 'widget', 'state', 'build', 'setState', 'apex',
  'android', 'flutter', 'web', 'math', 'forge', 'log', 'result', 'option', 'vec', 'str', 'map', 'set',
  'SceneBuilder', 'Layer', 'ContainerLayer', 'PictureLayer', 'TransformLayer', 'OpacityLayer', 'ClipLayer',
  'FlutterEmbedder', 'FlutterEngine', 'Renderer', 'RenderContext', 'RenderLoop', 'Scene',
  'PlatformChannel', 'MethodCall', 'MethodResponse', 'MethodError', 'BuildContext', 'RenderObject',
  'FutureHandle', 'FutureState', 'FutureKind', 'Executor', 'TaskGroup',
  'async.sleep', 'async.timeout', 'async.spawn', 'async.parallel', 'async.race', 'async.all',
  'async.any', 'async.then', 'async.catch', 'async.finally', 'async.cancel', 'async.is_cancelled',
  'async.channel', 'async.stream', 'async.yield', 'async.interval', 'async.retry',
  'ctx.text', 'ctx.button', 'ctx.column', 'ctx.row', 'ctx.container', 'ctx.image', 'ctx.list',
  'ctx.appbar', 'ctx.scaffold', 'ctx.widget', 'android.permissions', 'android.intent', 'android.storage'
];

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
    ...['.', ':']
  );

  context.subscriptions.push(provider);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
};
