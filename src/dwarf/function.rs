//! 函数符号解析 — DWARF 中 DW_TAG_subprogram 的收集与查询。

use gimli::EndianSlice;

use crate::dwarf::types::FunctionInfo;

/// 端序类型别名。
type Reader<'a> = EndianSlice<'a, gimli::RunTimeEndian>;

/// 从 DWARF 中收集所有函数信息。
pub(crate) fn collect_functions(
    dwarf: &gimli::Dwarf<Reader>,
) -> anyhow::Result<Vec<FunctionInfo>> {
    let mut functions = Vec::new();
    let mut iter = dwarf.units();
    while let Some(header) = iter
        .next()
        .map_err(|e| anyhow::anyhow!("unit iteration error: {e}"))?
    {
        let unit = dwarf
            .unit(header)
            .map_err(|e| anyhow::anyhow!("failed to load unit: {e}"))?;
        collect_functions_in_unit(dwarf, &unit, &mut functions)?;
    }
    Ok(functions)
}

fn collect_functions_in_unit(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    functions: &mut Vec<FunctionInfo>,
) -> anyhow::Result<()> {
    let mut cursor = unit.entries();
    while let Some((_depth, entry)) = cursor
        .next_dfs()
        .map_err(|e| anyhow::anyhow!("entry iteration error: {e}"))?
    {
        if entry.tag() != gimli::DW_TAG_subprogram {
            continue;
        }

        let name = match get_string_attr(dwarf, unit, entry, gimli::DW_AT_name)? {
            Some(n) => n,
            None => continue,
        };

        let low_pc = match get_addr_attr(dwarf, unit, entry, gimli::DW_AT_low_pc)? {
            Some(addr) => addr as u32,
            None => continue,
        };

        let high_pc = match get_high_pc(dwarf, unit, entry, low_pc as u64)? {
            Some(addr) => addr as u32,
            None => continue,
        };

        functions.push(FunctionInfo {
            name,
            low_addr: low_pc,
            high_addr: high_pc,
        });
    }
    Ok(())
}

/// 获取条目的字符串属性值。
fn get_string_attr(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    entry: &gimli::DebuggingInformationEntry<Reader>,
    attr_name: gimli::constants::DwAt,
) -> anyhow::Result<Option<String>> {
    let attr = match entry.attr(attr_name)? {
        Some(a) => a,
        None => return Ok(None),
    };
    let raw = dwarf.attr_string(unit, attr.value())?;
    Ok(Some(String::from_utf8_lossy(raw.slice()).into_owned()))
}

/// 获取条目的地址属性值（如 DW_AT_low_pc）。
fn get_addr_attr(
    _dwarf: &gimli::Dwarf<Reader>,
    _unit: &gimli::Unit<Reader>,
    entry: &gimli::DebuggingInformationEntry<Reader>,
    attr_name: gimli::constants::DwAt,
) -> anyhow::Result<Option<u64>> {
    let attr = match entry.attr(attr_name)? {
        Some(a) => a,
        None => return Ok(None),
    };
    match attr.value() {
        gimli::AttributeValue::Addr(addr) => Ok(Some(addr)),
        gimli::AttributeValue::Udata(addr) => {
            // Some compilers encode low_pc as udata on 32-bit targets
            Ok(Some(addr))
        }
        _ => Ok(None),
    }
}

/// 计算高地址：从 DW_AT_high_pc（地址值或偏移值）。
fn get_high_pc(
    _dwarf: &gimli::Dwarf<Reader>,
    _unit: &gimli::Unit<Reader>,
    entry: &gimli::DebuggingInformationEntry<Reader>,
    low_pc: u64,
) -> anyhow::Result<Option<u64>> {
    let attr = match entry.attr(gimli::DW_AT_high_pc)? {
        Some(a) => a,
        None => return Ok(None),
    };
    match attr.value() {
        gimli::AttributeValue::Addr(addr) => Ok(Some(addr)),
        gimli::AttributeValue::Udata(len) => Ok(Some(low_pc + len)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    // 集成测试在 mod.rs
}
