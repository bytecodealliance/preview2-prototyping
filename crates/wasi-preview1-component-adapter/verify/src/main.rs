use anyhow::{bail, Result};
use std::env;
use wasmparser::*;

const ALLOWED_IMPORT_MODULES: &[&str] = &[
    "wall-clock",
    "monotonic-clock",
    "timezone",
    "filesystem",
    "instance-network",
    "ip-name-lookup",
    "network",
    "tcp-create-socket",
    "tcp",
    "udp-create-socket",
    "udp",
    "random",
    "poll",
    "streams",
    "environment",
    "preopens",
    "exit",
    "canonical_abi",
    "__main_module__",
];

fn main() -> Result<()> {
    let file = env::args()
        .nth(1)
        .expect("must pass wasm file as an argument");
    let wasm = wat::parse_file(&file)?;

    let mut validator = Validator::new();
    for payload in Parser::new(0).parse_all(&wasm) {
        let payload = payload?;
        validator.payload(&payload)?;
        match payload {
            Payload::Version { encoding, .. } => {
                if encoding != Encoding::Module {
                    bail!("adapter must be a core wasm module, not a component");
                }
            }
            Payload::End(_) => {}
            Payload::TypeSection(_) => {}
            Payload::ImportSection(s) => {
                for i in s {
                    let i = i?;
                    match i.ty {
                        TypeRef::Func(_) => {
                            if !ALLOWED_IMPORT_MODULES.contains(&i.module) {
                                bail!("import from unknown module `{}`", i.module);
                            }
                        }
                        TypeRef::Table(_) => bail!("should not import table"),
                        TypeRef::Global(_) => bail!("should not import globals"),
                        TypeRef::Memory(_) => {}
                        TypeRef::Tag(_) => bail!("unsupported `tag` type"),
                    }
                }
            }
            Payload::TableSection(_) => {}
            Payload::MemorySection(_) => {
                bail!("preview1.wasm should import memory");
            }
            Payload::GlobalSection(_) => {}

            Payload::ExportSection(_) => {}

            Payload::FunctionSection(_) => {}

            Payload::CodeSectionStart { .. } => {}
            Payload::CodeSectionEntry(_) => {}
            Payload::CustomSection(_) => {}

            // sections that shouldn't appear in the specially-crafted core wasm
            // adapter self we're processing
            Payload::DataCountSection { .. }
            | Payload::ElementSection(_)
            | Payload::DataSection(_)
            | Payload::StartSection { .. }
            | Payload::TagSection(_)
            | Payload::UnknownSection { .. } => {
                bail!("unsupported section {payload:?} found in preview1.wasm")
            }

            // component-model related things that shouldn't show up
            Payload::ModuleSection { .. }
            | Payload::ComponentSection { .. }
            | Payload::InstanceSection(_)
            | Payload::ComponentInstanceSection(_)
            | Payload::ComponentAliasSection(_)
            | Payload::ComponentCanonicalSection(_)
            | Payload::ComponentStartSection(_)
            | Payload::ComponentImportSection(_)
            | Payload::CoreTypeSection(_)
            | Payload::ComponentExportSection(_)
            | Payload::ComponentTypeSection(_) => {
                bail!("component section found in preview1.wasm")
            }
        }
    }

    Ok(())
}
