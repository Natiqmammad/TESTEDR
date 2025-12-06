use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

const ELF_HEADER_SIZE: usize = 64;
const PROGRAM_HEADER_SIZE: usize = 56;
const LOAD_ADDR: u64 = 0x4000_0000;

pub fn write_elf(code: &[u8], path: &Path) -> Result<()> {
    let mut file = File::create(path)
        .with_context(|| format!("failed to create executable {}", path.display()))?;

    let text_offset = (ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE) as u64;
    let entry = LOAD_ADDR + text_offset;
    let file_size = (ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE + code.len()) as u64;

    let mut elf = Vec::with_capacity(file_size as usize);

    elf.extend_from_slice(b"\x7FELF"); // magic
    elf.push(2); // 64-bit
    elf.push(1); // little-endian
    elf.push(1); // ELF version
    elf.push(0); // OS ABI
    elf.push(0); // ABI version
    elf.extend_from_slice(&[0u8; 7]);
    elf.extend_from_slice(&u16_to_le(2)); // ET_EXEC
    elf.extend_from_slice(&u16_to_le(0x3E)); // x86_64
    elf.extend_from_slice(&u32_to_le(1)); // version
    elf.extend_from_slice(&u64_to_le(entry));
    elf.extend_from_slice(&u64_to_le(ELF_HEADER_SIZE as u64)); // program header offset
    elf.extend_from_slice(&u64_to_le(0)); // section header offset
    elf.extend_from_slice(&u32_to_le(0)); // flags
    elf.extend_from_slice(&u16_to_le(ELF_HEADER_SIZE as u16));
    elf.extend_from_slice(&u16_to_le(PROGRAM_HEADER_SIZE as u16));
    elf.extend_from_slice(&u16_to_le(1)); // number program headers
    elf.extend_from_slice(&u16_to_le(0)); // shentsize
    elf.extend_from_slice(&u16_to_le(0)); // shnum
    elf.extend_from_slice(&u16_to_le(0)); // shstrndx

    // Program header
    elf.extend_from_slice(&u32_to_le(1)); // PT_LOAD
    elf.extend_from_slice(&u32_to_le(5)); // PF_X | PF_R
    elf.extend_from_slice(&u64_to_le(0)); // file offset
    elf.extend_from_slice(&u64_to_le(LOAD_ADDR)); // vaddr
    elf.extend_from_slice(&u64_to_le(LOAD_ADDR)); // paddr
    elf.extend_from_slice(&u64_to_le(file_size));
    elf.extend_from_slice(&u64_to_le(file_size));
    elf.extend_from_slice(&u64_to_le(0x1000)); // alignment

    // Ensure code starts at text_offset
    while elf.len() < ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE {
        elf.push(0);
    }
    elf.extend_from_slice(code);

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

fn u64_to_le(value: u64) -> [u8; 8] {
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
