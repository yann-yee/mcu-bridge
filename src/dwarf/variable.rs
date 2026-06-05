//! 变量符号解析 — DWARF 中 DW_TAG_variable 的收集与查询。

use gimli::EndianSlice;

use crate::dwarf::types::VariableInfo;

/// 端序类型别名。
type Reader<'a> = EndianSlice<'a, gimli::RunTimeEndian>;

/// 从 DWARF 中收集所有全局/静态变量信息。
pub(crate) fn collect_variables(dwarf: &gimli::Dwarf<Reader>) -> anyhow::Result<Vec<VariableInfo>> {
    let mut variables = Vec::new();
    let mut iter = dwarf.units();
    while let Some(header) = iter
        .next()
        .map_err(|e| anyhow::anyhow!("unit iteration error: {e}"))?
    {
        let unit = dwarf
            .unit(header)
            .map_err(|e| anyhow::anyhow!("failed to load unit: {e}"))?;
        collect_variables_in_unit(dwarf, &unit, &mut variables)?;
    }
    Ok(variables)
}

fn collect_variables_in_unit(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    variables: &mut Vec<VariableInfo>,
) -> anyhow::Result<()> {
    let mut cursor = unit.entries();
    while let Some((_depth, entry)) = cursor
        .next_dfs()
        .map_err(|e| anyhow::anyhow!("entry iteration error: {e}"))?
    {
        if entry.tag() != gimli::DW_TAG_variable {
            continue;
        }

        let name = match get_string_attr(dwarf, unit, entry, gimli::DW_AT_name)? {
            Some(n) => n,
            None => continue,
        };

        let addr = match get_global_addr(entry) {
            Some(a) => a as u32,
            None => continue,
        };

        let size = resolve_size_from_type(dwarf, unit, entry).unwrap_or(4);

        variables.push(VariableInfo {
            name,
            addr,
            size,
            type_name: None,
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

/// 从 DW_AT_location 提取全局变量绝对地址（仅支持 DW_OP_addr）。
fn get_global_addr(entry: &gimli::DebuggingInformationEntry<Reader>) -> Option<u64> {
    let attr = entry.attr(gimli::DW_AT_location).ok()??;
    match attr.value() {
        gimli::AttributeValue::Exprloc(ref expr) => {
            // Expression wraps the Reader, access inner data
            let bytes = expr.0.slice();
            if bytes.is_empty() || bytes[0] != 0x03 {
                return None;
            }
            let addr_bytes = &bytes[1..];
            if addr_bytes.len() == 4 {
                Some(u64::from(u32::from_ne_bytes([
                    addr_bytes[0],
                    addr_bytes[1],
                    addr_bytes[2],
                    addr_bytes[3],
                ])))
            } else if addr_bytes.len() == 8 {
                Some(u64::from_ne_bytes([
                    addr_bytes[0],
                    addr_bytes[1],
                    addr_bytes[2],
                    addr_bytes[3],
                    addr_bytes[4],
                    addr_bytes[5],
                    addr_bytes[6],
                    addr_bytes[7],
                ]))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 通过变量的 DW_AT_type 引用解析字节大小。
fn resolve_size_from_type(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    entry: &gimli::DebuggingInformationEntry<Reader>,
) -> Option<u32> {
    let type_attr = entry.attr(gimli::DW_AT_type).ok()??;
    let debug_info_offset = match type_attr.value() {
        gimli::AttributeValue::DebugInfoRef(offset) => offset,
        _ => return None,
    };

    resolve_entry_byte_size(dwarf, unit, &debug_info_offset)
}

/// 在指定的 DebugInfoOffset 处解析 DIE 的 byte_size。
#[expect(clippy::only_used_in_recursion)]
fn resolve_entry_byte_size(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    debug_info_offset: &gimli::DebugInfoOffset<usize>,
) -> Option<u32> {
    let unit_offset = debug_info_offset.to_unit_offset(&unit.header)?;

    let entries = unit.entries_at_offset(unit_offset).ok()?;
    let type_entry = entries.current()?;

    // 直接读取 byte_size
    if let Some(attr) = type_entry.attr(gimli::DW_AT_byte_size).ok()?
        && let gimli::AttributeValue::Udata(size) = attr.value()
    {
        return Some(size as u32);
    }

    // 指针类型默认 4 字节
    if type_entry.tag() == gimli::DW_TAG_pointer_type {
        return Some(4);
    }

    // typedef — 跟随其 DW_AT_type
    if type_entry.tag() == gimli::DW_TAG_typedef
        && let Some(attr) = type_entry.attr(gimli::DW_AT_type).ok()?
        && let gimli::AttributeValue::DebugInfoRef(next_offset) = attr.value()
    {
        return resolve_entry_byte_size(dwarf, unit, &next_offset);
    }

    None
}
