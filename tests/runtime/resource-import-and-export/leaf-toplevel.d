module leaf_toplevel;

import wit.test.resource_import_and_export.leaf_toplevel;

import wit.common;

@witExport("$root", "toplevel-export")
Thing toplevelExport(Thing input) {
    // `input` not dropped b/c ownership transferred
    // via return

    return input;
}

alias Exports = wit.test.resource_import_and_export.leaf_toplevel.Exports!(
    toplevelExport
);
