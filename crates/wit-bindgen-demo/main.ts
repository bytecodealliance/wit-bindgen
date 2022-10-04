import { Demo, Options } from './demo.js';
import * as browser from './browser.js';

class Editor {
  input: HTMLTextAreaElement;
  language: HTMLSelectElement;
  mode: HTMLSelectElement;
  files: HTMLSelectElement
  rustUnchecked: HTMLInputElement;
  wasmtimeTracing: HTMLInputElement;
  wasmtimeCustomError: HTMLInputElement;
  generatedFiles: Record<string, string>;
  demo: Demo;
  options: Options;
  rerender: number | null;
  inputEditor: AceAjax.Editor;
  outputEditor: AceAjax.Editor;
  outputHtml: HTMLDivElement;

  constructor() {
    this.input = document.getElementById('input-raw') as HTMLTextAreaElement;
    this.language = document.getElementById('language-select') as HTMLSelectElement;
    this.mode = document.getElementById('mode-select') as HTMLSelectElement;
    this.files = document.getElementById('file-select') as HTMLSelectElement;
    this.rustUnchecked = document.getElementById('rust-unchecked') as HTMLInputElement;
    this.wasmtimeTracing = document.getElementById('wasmtime-tracing') as HTMLInputElement;
    this.wasmtimeCustomError = document.getElementById('wasmtime-custom-error') as HTMLInputElement;
    this.outputHtml = document.getElementById('html-output') as HTMLDivElement;

    this.inputEditor = ace.edit("input");
    this.outputEditor = ace.edit("output");
    this.inputEditor.setValue(this.input.value);
    this.inputEditor.clearSelection();
    this.outputEditor.setReadOnly(true);
    this.inputEditor.setOption("useWorker", false);
    this.outputEditor.setOption("useWorker", false);

    this.generatedFiles = {};
    this.demo = new Demo();
    this.options = {
      rustUnchecked: false,
      wasmtimeTracing: false,
      wasmtimeCustomError: false,
      import: false,
    };
    this.rerender = null;
  }

  async instantiate() {
    const imports = {};
    const obj = {
      log: console.log,
      error: console.error,
    };
    browser.addBrowserToImports(imports, obj, name => this.demo.instance.exports[name]);
    await this.demo.instantiate(fetch('./demo.wasm'), imports);
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
      this.options.rustUnchecked = this.rustUnchecked.checked;
      this.render();
    });

    this.wasmtimeTracing.addEventListener('change', () => {
      this.options.wasmtimeTracing = this.wasmtimeTracing.checked;
      this.render();
    });
    this.wasmtimeCustomError.addEventListener('change', () => {
      this.options.wasmtimeCustomError = this.wasmtimeCustomError.checked;
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

    const wit = this.inputEditor.getValue();
    const is_import = this.mode.value === 'import';
    let lang;
    switch (this.language.value) {
      case "js":
      case "rust":
      case "java":
      case "wasmtime":
      case "wasmtime-py":
      case "c":
      case "markdown":
        lang = this.language.value;
        break;
      default: return;
    }
    this.options.import = is_import;
    const result = this.demo.render(lang, wit, this.options);
    if (result.tag === 'err') {
      this.outputEditor.setValue(result.val);
      this.outputEditor.clearSelection();
      this.showOutputEditor();
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

  showOutputEditor() {
    this.outputHtml.style.display = 'none';
    document.getElementById('output').style.display = 'block';
  }

  showOutputHtml() {
    this.outputHtml.style.display = 'block';
    document.getElementById('output').style.display = 'none';
  }

  updateSelectedFile() {
    if (this.files.value.endsWith('.html')) {
      const html = this.generatedFiles[this.files.value];
      this.outputHtml.innerHTML = html;
      this.showOutputHtml();
      return;
    }

    this.showOutputEditor();
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
    else if (this.files.value.endsWith('.md'))
      this.outputEditor.session.setMode("ace/mode/markdown");
    else if (this.files.value.endsWith('.py'))
      this.outputEditor.session.setMode("ace/mode/python");
    else if (this.files.value.endsWith('.java'))
      this.outputEditor.session.setMode("ace/mode/java");
    else
      this.outputEditor.session.setMode(null);
  }
}


(new Editor()).instantiate()
