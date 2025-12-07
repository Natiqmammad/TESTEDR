use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use super::emitter::{MachineCode, Patch};

pub const ELF_HEADER_SIZE: usize = 52;
pub const PROGRAM_HEADER_SIZE: usize = 32;
pub const LOAD_ADDR: u32 = 0x0804_8000;

pub fn write_elf(machine: &MachineCode, path: &Path) -> Result<()> {
    let mut file = File::create(path)
        .with_context(|| format!("failed to create executable {}", path.display()))?;

    let mut text = machine.code.clone();
    let mut string_offsets = Vec::new();
    for data in &machine.strings {
        let offset = text.len();
        string_offsets.push(offset);
        text.extend_from_slice(data);
    }

    let text_base = (ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE) as u32;
    patch_strings(&mut text, text_base, &string_offsets, &machine.patches)?;

    let entry = LOAD_ADDR + text_base;
    let file_size = (ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE + text.len()) as u32;

    let mut elf = Vec::with_capacity(file_size as usize);

    elf.extend_from_slice(b"\x7FELF");
    elf.push(1); // 32-bit
    elf.push(1); // little endian
    elf.push(1); // version
    elf.push(0); // ABI
    elf.push(0);
    elf.extend_from_slice(&[0u8; 7]);
    elf.extend_from_slice(&u16_to_le(2)); // type ET_EXEC
    elf.extend_from_slice(&u16_to_le(0x03)); // machine x86
    elf.extend_from_slice(&u32_to_le(1)); // version
    elf.extend_from_slice(&u32_to_le(entry));
    elf.extend_from_slice(&u32_to_le(ELF_HEADER_SIZE as u32));
    elf.extend_from_slice(&u32_to_le(0));
    elf.extend_from_slice(&u16_to_le(ELF_HEADER_SIZE as u16));
    elf.extend_from_slice(&u16_to_le(PROGRAM_HEADER_SIZE as u16));
    elf.extend_from_slice(&u16_to_le(1)); // phnum
    elf.extend_from_slice(&u16_to_le(0)); // shentsize
    elf.extend_from_slice(&u16_to_le(0)); // shnum
    elf.extend_from_slice(&u16_to_le(0)); // shstrndx

    // Program header
    elf.extend_from_slice(&u32_to_le(1)); // PT_LOAD
    elf.extend_from_slice(&u32_to_le(0)); // offset
    elf.extend_from_slice(&u32_to_le(LOAD_ADDR));
    elf.extend_from_slice(&u32_to_le(LOAD_ADDR));
    elf.extend_from_slice(&u32_to_le(file_size));
    elf.extend_from_slice(&u32_to_le(file_size));
    elf.extend_from_slice(&u32_to_le(0x1000)); // align

    while elf.len() < ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE {
        elf.push(0);
    }
    elf.extend_from_slice(&text);

    file.write_all(&elf)?;
    set_executable_permissions(path)?;
    Ok(())
}

fn u16_to_le(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

fn u32_to_le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

#[cfg(unix)]
fn set_executable_permissions(path: &Path) -> Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn patch_strings(
    text: &mut [u8],
    text_base: u32,
    string_offsets: &[usize],
    patches: &[Patch],
) -> Result<()> {
    for patch in patches {
        let string_offset = *string_offsets
            .get(patch.string_id as usize)
            .ok_or_else(|| anyhow!("invalid string id {}", patch.string_id))?;
        let addr = LOAD_ADDR + text_base + string_offset as u32;
        let end = patch.offset + 4;
        let slot = text
            .get_mut(patch.offset..end)
            .ok_or_else(|| anyhow!("invalid patch offset"))?;
        slot.copy_from_slice(&addr.to_le_bytes());
    }
    Ok(())
}
