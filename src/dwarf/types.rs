//! DWARF 解析共享类型定义。
//!
//! 所有类型均为 owned，不借用外部数据，便于在 DwarfResolver 中存储。

/// 函数符号信息。
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub low_addr: u32,
    pub high_addr: u32,
}

/// 全局/静态变量信息。
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub addr: u32,
    pub size: u32,
    pub type_name: Option<String>,
}
