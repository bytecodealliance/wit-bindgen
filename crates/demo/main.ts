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

const input = document.getElementById('input-raw') as HTMLTextAreaElement;
const language = document.getElementById('language-select') as HTMLSelectElement;
const mode = document.getElementById('mode-select') as HTMLSelectElement;
const files = document.getElementById('file-select') as HTMLSelectElement;
const rustUnchecked = document.getElementById('rust-unchecked') as HTMLInputElement;
const wasmtimeTracing = document.getElementById('wasmtime-tracing') as HTMLInputElement;
const wasmtimeAsync = document.getElementById('wasmtime-async') as HTMLInputElement;

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

  for (let div of document.querySelectorAll('.lang-configure')) {
    (div as HTMLDivElement).style.display = 'none';
  }

  const config = document.getElementById(`configure-${language.value}`);
  config.style.display = 'inline-block';

  const witx = inputEditor.getValue();
  const is_import = mode.value === 'import';
  let result;
  switch (language.value) {
    case "js":
      result = wasm.render_js(witx, is_import);
      break;
    case "rust":
      result = wasm.render_rust(witx, is_import, rustUnchecked.checked);
      break;
    case "wasmtime":
      let async_;
      if (wasmtimeAsync.checked)
        async_ = { tag: 'all' };
      else
        async_ = { tag: 'none' };
      result = wasm.render_wasmtime(witx, is_import, wasmtimeTracing.checked, async_);
      break;
    default: return;
  }

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
  else
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
rustUnchecked.addEventListener('change', render);
wasmtimeTracing.addEventListener('change', render);
wasmtimeAsync.addEventListener('change', render);
files.addEventListener('change', updateSelectedFile);


render()
