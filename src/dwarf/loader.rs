//! ELF 加载与 DWARF section 提取。
//!
//! 生命周期管理：ELF 数据在栈上借用于函数内部，全部解析为 owned 数据后释放。

use std::path::Path;

use gimli::{EndianSlice, RunTimeEndian};
use object::Object;
use object::ObjectSection;

/// 加载 ELF 并对 DWARF 执行闭包，返回闭包产出的 owned 结果。
///
/// 闭包接收 `&gimli::Dwarf<EndianSlice<RunTimeEndian>>`（借用于栈上数据），
/// 闭包内可遍历 DIE 树收集为 owned 类型。
pub(crate) fn with_dwarf<T>(
    path: &Path,
    f: impl FnOnce(&gimli::Dwarf<EndianSlice<RunTimeEndian>>) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let elf_data = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read ELF file '{}': {e}", path.display()))?;

    let endian = detect_endian(&elf_data)?;

    let file = object::read::File::parse(&elf_data[..])
        .map_err(|e| anyhow::anyhow!("failed to parse ELF: {e}"))?;

    let dwarf = build_dwarf_from_file(&file, endian)?;
    f(&dwarf)
}

/// 检测 ELF 字节序。
fn detect_endian(data: &[u8]) -> anyhow::Result<gimli::RunTimeEndian> {
    if data.len() < 5 {
        anyhow::bail!("file too small to be a valid ELF");
    }
    match data[4] {
        1 => Ok(RunTimeEndian::Little),
        2 => Ok(RunTimeEndian::Big),
        other => anyhow::bail!("unknown ELF endianness byte: {other}"),
    }
}

/// 从已解析的 object::read::File 构建 gimli::Dwarf。
fn build_dwarf_from_file<'data>(
    file: &'data object::read::File<'data>,
    endian: RunTimeEndian,
) -> anyhow::Result<gimli::Dwarf<EndianSlice<'data, RunTimeEndian>>> {
    let load_section = |id: gimli::SectionId| -> EndianSlice<'data, RunTimeEndian> {
        match file.section_by_name_bytes(id.name().as_bytes()) {
            Some(section) => match section.data() {
                Ok(data) => EndianSlice::new(data, endian),
                Err(_) => EndianSlice::new(&[], endian),
            },
            None => EndianSlice::new(&[], endian),
        }
    };

    let dwarf_result: Result<gimli::Dwarf<EndianSlice<'data, RunTimeEndian>>, gimli::Error> =
        gimli::Dwarf::load(|section_id| -> Result<EndianSlice<'data, RunTimeEndian>, gimli::Error> {
            Ok(load_section(section_id))
        });
    dwarf_result.map_err(|e| anyhow::anyhow!("failed to load DWARF sections: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_load_nonexistent_file() {
        let result = with_dwarf(Path::new("/nonexistent/file.elf"), |_| Ok(()));
        assert!(result.is_err());
    }
}
