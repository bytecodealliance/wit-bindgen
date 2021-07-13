import { Demo, Lang, Config } from './demo/bindings.js';
import * as browser from './browser/bindings.js';

class Editor {
  input: HTMLTextAreaElement;
  language: HTMLSelectElement;
  mode: HTMLSelectElement;
  files: HTMLSelectElement
  rustUnchecked: HTMLInputElement;
  wasmtimeTracing: HTMLInputElement;
  wasmtimeAsync: HTMLInputElement;
  wasmtimeCustomError: HTMLInputElement;
  generatedFiles: Record<string, string>;
  demo: Demo;
  config: Config | null;
  rerender: number | null;
  inputEditor: AceAjax.Editor;
  outputEditor: AceAjax.Editor;

  constructor() {
    this.input = document.getElementById('input-raw') as HTMLTextAreaElement;
    this.language = document.getElementById('language-select') as HTMLSelectElement;
    this.mode = document.getElementById('mode-select') as HTMLSelectElement;
    this.files = document.getElementById('file-select') as HTMLSelectElement;
    this.rustUnchecked = document.getElementById('rust-unchecked') as HTMLInputElement;
    this.wasmtimeTracing = document.getElementById('wasmtime-tracing') as HTMLInputElement;
    this.wasmtimeAsync = document.getElementById('wasmtime-async') as HTMLInputElement;
    this.wasmtimeCustomError = document.getElementById('wasmtime-custom-error') as HTMLInputElement;

    this.inputEditor = ace.edit("input");
    this.outputEditor = ace.edit("output");
    this.inputEditor.setValue(this.input.value);
    this.inputEditor.clearSelection();
    this.outputEditor.setReadOnly(true);
    this.inputEditor.setOption("useWorker", false);
    this.outputEditor.setOption("useWorker", false);

    this.generatedFiles = {};
    this.demo = new Demo();
    this.config = null;
    this.rerender = null;
  }

  async instantiate() {
    const imports = {};
    const obj = {
      log: console.log,
      error: console.error,
    };
    browser.add_browser_to_imports(imports, obj, name => this.demo.instance.exports[name]);
    await this.demo.instantiate(fetch('./demo.wasm'), imports);
    this.config = this.demo.config_new();
    this.installListeners();
    this.render();
  }

  installListeners() {
    this.inputEditor.on('change', () => {
      this.input.value = this.inputEditor.getValue();
      if (this.rerender !== null)
        clearTimeout(this.rerender);
      this.rerender = setTimeout(() => this.render(), 500);
    });

    this.language.addEventListener('change', () => this.render());
    this.mode.addEventListener('change', () => this.render());

    this.rustUnchecked.addEventListener('change', () => {
      this.demo.set_rust_unchecked(this.config, this.rustUnchecked.checked);
      this.render();
    });

    this.wasmtimeTracing.addEventListener('change', () => {
      this.demo.set_wasmtime_tracing(this.config, this.wasmtimeTracing.checked);
      this.render();
    });
    this.wasmtimeAsync.addEventListener('change', () => {
      let async_;
      if (this.wasmtimeAsync.checked)
        async_ = { tag: 'all' };
      else
        async_ = { tag: 'none' };
      this.demo.set_wasmtime_async(this.config, async_);
      this.render();
    });
    this.wasmtimeCustomError.addEventListener('change', () => {
      this.demo.set_wasmtime_custom_error(this.config, this.wasmtimeCustomError.checked);
      this.render();
    });
    this.files.addEventListener('change', () => this.updateSelectedFile());
  }


  render() {
    for (let div of document.querySelectorAll('.lang-configure')) {
      (div as HTMLDivElement).style.display = 'none';
    }

    const config = document.getElementById(`configure-${this.language.value}`);
    config.style.display = 'inline-block';

    const witx = this.inputEditor.getValue();
    const is_import = this.mode.value === 'import';
    let lang;
    switch (this.language.value) {
      case "js": lang = Lang.Js; break;
      case "rust": lang = Lang.Rust; break;
      case "wasmtime": lang = Lang.Wasmtime; break;
      case "c": lang = Lang.C; break;
      default: return;
    }
    const result = this.demo.render(this.config, lang, witx, is_import);
    if (result.tag === 'err') {
      this.outputEditor.setValue(result.val);
      this.outputEditor.clearSelection();
      return;
    }
    this.generatedFiles = {};
    const selectedFile = this.files.value;
    this.files.options.length = 0;
    for (let i = 0; i < result.val.length; i++) {
      const name = result.val[i][0];
      const contents = result.val[i][1];
      this.files.options[i] = new Option(name, name);
      this.generatedFiles[name] = contents;
    }
    if (selectedFile in this.generatedFiles)
      this.files.value = selectedFile;

    this.updateSelectedFile();
  }

  updateSelectedFile() {
    this.outputEditor.setValue(this.generatedFiles[this.files.value]);
    this.outputEditor.clearSelection();
    if (this.files.value.endsWith('.d.ts'))
      this.outputEditor.session.setMode("ace/mode/typescript");
    else if (this.files.value.endsWith('.js'))
      this.outputEditor.session.setMode("ace/mode/javascript");
    else if (this.files.value.endsWith('.rs'))
      this.outputEditor.session.setMode("ace/mode/rust");
    else if (this.files.value.endsWith('.c'))
      this.outputEditor.session.setMode("ace/mode/c_cpp");
    else if (this.files.value.endsWith('.h'))
      this.outputEditor.session.setMode("ace/mode/c_cpp");
    else
      this.outputEditor.session.setMode(null);
  }
}


(new Editor()).instantiate()
