use crate::{decode_interface_component, ComponentEncoder};
use anyhow::{Context, Result};
use wasmparser::{Parser, Payload};

/// Transcode a core Module containing Component types in custom sections into a Component.
pub fn transcode(module: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ComponentEncoder::default().module(&module).validate(true);

    let mut interface = None;
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for payload in Parser::new(0).parse_all(&module) {
        match payload.context("decoding item in module")? {
            Payload::CustomSection(cs) => {
                if let Some(export) = cs.name().strip_prefix("component-type:export:") {
                    let mut i = decode_interface_component(cs.data()).with_context(|| {
                        format!("decoding component-type in export section {}", export)
                    })?;
                    i.name = export.to_owned();
                    interface = Some(i);
                } else if let Some(import) = cs.name().strip_prefix("component-type:import:") {
                    let mut interface =
                        decode_interface_component(cs.data()).with_context(|| {
                            format!("decoding component-type in import section {}", import)
                        })?;
                    interface.name = import.to_owned();
                    imports.push(interface);
                } else if let Some(export_instance) =
                    cs.name().strip_prefix("component-type:export-instance:")
                {
                    let mut interface =
                        decode_interface_component(cs.data()).with_context(|| {
                            format!(
                                "decoding component-type in export-instance section {}",
                                export_instance
                            )
                        })?;
                    interface.name = export_instance.to_owned();
                    exports.push(interface);
                }
            }
            _ => {}
        }
    }

    encoder = encoder.imports(&imports).exports(&exports);
    if let Some(interface) = &interface {
        encoder = encoder.interface(&interface);
    }

    encoder.encode()
}
