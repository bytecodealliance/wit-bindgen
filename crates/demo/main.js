import * as demo from './demo/bindings.js';
import * as browser from './browser/bindings.js';

const imports = {};
const obj = {
  log: console.log,
  error: console.error,
};
let instance = null;
browser.add_browser_to_imports(imports, obj, name => instance.exports[name]);
const wasmPromise = demo.Demo.instantiate(fetch('./demo.wasm'), imports)
  .then(demo => {
    instance = demo.instance;
    return demo;
  });

const input = document.getElementById('input-raw');
const language = document.getElementById('language-select');
const mode = document.getElementById('mode-select');
const files = document.getElementById('file-select');

const inputEditor = ace.edit("input");
const outputEditor = ace.edit("output");
inputEditor.setValue(input.value);
inputEditor.clearSelection();
outputEditor.setReadOnly(true);
inputEditor.setOption("useWorker", false);
outputEditor.setOption("useWorker", false);

let generatedFiles = {};

async function render() {
  const wasm = await wasmPromise;
  let lang;
  switch (language.value) {
    case "js": lang = demo.Lang.Js; break;
    case "rust": lang = demo.Lang.Rust; break;
    case "wasmtime": lang = demo.Lang.Wasmtime; break;
    default: return;
  }
  const result = wasm.render(inputEditor.getValue(), lang, mode.value === 'import');
  if (result.tag === 'err') {
    outputEditor.setValue(result.val);
    outputEditor.clearSelection();
    return;
  }
  generatedFiles = {};
  const selectedFile = files.value;
  files.options.length = 0;
  for (let i = 0; i < result.val.length; i++) {
    const name = result.val[i][0];
    const contents = result.val[i][1];
    files.options[i] = new Option(name, name);
    generatedFiles[name] = contents;
  }
  if (selectedFile in generatedFiles)
    files.value = selectedFile;

  updateSelectedFile();
}

function updateSelectedFile() {
  outputEditor.setValue(generatedFiles[files.value]);
  outputEditor.clearSelection();
  if (files.value.endsWith('.d.ts'))
    outputEditor.session.setMode("ace/mode/typescript");
  else if (files.value.endsWith('.js'))
    outputEditor.session.setMode("ace/mode/javascript");
  else if (files.value.endsWith('.rs'))
    outputEditor.session.setMode("ace/mode/rust");
  else if (files.value.endsWith('.rs'))
    outputEditor.session.setMode(null);

}

let rerender = null;
inputEditor.on('change', function(){
  input.value = inputEditor.getValue();
  if (rerender !== null)
    clearTimeout(rerender);
  rerender = setTimeout(render, 500);
});

language.addEventListener('change', render);
mode.addEventListener('change', render);
files.addEventListener('change', updateSelectedFile);


render()
