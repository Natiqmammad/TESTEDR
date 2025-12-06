use anyhow::Result;

use super::lower::LoweredModule;

pub fn emit_x86_64(_lowered: &LoweredModule) -> Result<Vec<u8>> {
    // xor edi, edi
    // mov eax, 60
    // syscall
    Ok(vec![0x31, 0xff, 0xb8, 0x3c, 0x00, 0x00, 0x00, 0x0f, 0x05])
}
