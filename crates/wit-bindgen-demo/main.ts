import { render, Options } from './demo.js';

class Editor {
  input: HTMLTextAreaElement;
  language: HTMLSelectElement;
  files: HTMLSelectElement
  rustUnchecked: HTMLInputElement;
  jsCompat: HTMLInputElement;
  jsInstantiation: HTMLInputElement;
  generatedFiles: Record<string, string>;
  options: Options;
  rerender: number | null;
  inputEditor: AceAjax.Editor;
  outputEditor: AceAjax.Editor;
  outputHtml: HTMLDivElement;

  constructor() {
    this.input = document.getElementById('input-raw') as HTMLTextAreaElement;
    this.language = document.getElementById('language-select') as HTMLSelectElement;
    this.files = document.getElementById('file-select') as HTMLSelectElement;
    this.rustUnchecked = document.getElementById('rust-unchecked') as HTMLInputElement;
    this.jsCompat = document.getElementById('js-compat') as HTMLInputElement;
    this.jsInstantiation = document.getElementById('js-instantiation') as HTMLInputElement;
    this.outputHtml = document.getElementById('html-output') as HTMLDivElement;

    this.inputEditor = ace.edit("input");
    this.outputEditor = ace.edit("output");
    this.inputEditor.setValue(this.input.value);
    this.inputEditor.clearSelection();
    this.outputEditor.setReadOnly(true);
    this.inputEditor.setOption("useWorker", false);
    this.outputEditor.setOption("useWorker", false);

    this.generatedFiles = {};
    this.options = {
      rustUnchecked: false,
      jsCompat: false,
      jsInstantiation: false,
    };
    this.rerender = null;
  }

  init() {
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

    this.rustUnchecked.addEventListener('change', () => {
      this.options.rustUnchecked = this.rustUnchecked.checked;
      this.render();
    });

    this.jsCompat.addEventListener('change', () => {
      this.options.jsCompat = this.jsCompat.checked;
      this.render();
    });

    this.jsInstantiation.addEventListener('change', () => {
      this.options.jsInstantiation = this.jsInstantiation.checked;
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
    let lang;
    switch (this.language.value) {
      case "js":
      case "rust":
      case "java":
      case "c":
      case "markdown":
        lang = this.language.value;
        break;
      default: return;
    }
    try {
      const results = render(lang, wit, this.options);
      this.generatedFiles = {};
      const selectedFile = this.files.value;
      this.files.options.length = 0;
      for (let i = 0; i < results.length; i++) {
        const name = results[i][0];
        const contents = results[i][1];
        this.files.options[i] = new Option(name, name);
        this.generatedFiles[name] = contents;
      }
      if (selectedFile in this.generatedFiles)
        this.files.value = selectedFile;

      this.updateSelectedFile();
    } catch (e) {
      this.outputEditor.setValue(e.payload);
      this.outputEditor.clearSelection();
      this.showOutputEditor();
    }
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


(new Editor()).init()
