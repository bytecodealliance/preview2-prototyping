use anyhow::{bail, Result};
use std::env;
use wasmparser::*;

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
                            if i.module != "wall-clock"
                                && i.module != "monotonic-clock"
                                && i.module != "instance-wall-clock"
                                && i.module != "instance-monotonic-clock"
                                && i.module != "timezone"
                                && i.module != "filesystem"
                                && i.module != "instance-network"
                                && i.module != "ip-name-lookup"
                                && i.module != "network"
                                && i.module != "tcp-create-socket"
                                && i.module != "tcp"
                                && i.module != "udp-create-socket"
                                && i.module != "udp"
                                && i.module != "random"
                                && i.module != "poll"
                                && i.module != "streams"
                                && i.module != "environment"
                                && i.module != "environment-preopens"
                                && i.module != "exit"
                                && i.module != "stderr"
                                && i.module != "canonical_abi"
                                && i.module != "__main_module__"
                            {
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
                bail!("unsupported section found in preview1.wasm")
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
