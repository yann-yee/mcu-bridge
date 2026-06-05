//! DWARF 符号解析 — ELF 中 DWARF 调试信息的加载与查询。
//!
//! 核心结构体 [`DwarfResolver`] 提供函数名↔地址双向解析、
//! 全局变量名→地址解析、地址→行号解析等能力。
//!
//! # 生命周期
//!
//! 所有解析结果均为 owned 数据，`DwarfResolver` 不借用外部数据。
//! 构造时一次性加载 ELF 并遍历 DIE 树，之后可任意共享。

pub(crate) mod function;
pub(crate) mod loader;
pub(crate) mod types;
pub(crate) mod variable;

use std::collections::HashMap;
use std::path::Path;

use function::collect_functions;
use types::{FunctionInfo, VariableInfo};
use variable::collect_variables;

/// DWARF 符号解析器。
///
/// 从 ELF 文件中加载 DWARF 调试信息，构建函数和变量的可查询索引。
#[derive(Clone, Debug)]
pub struct DwarfResolver {
    /// 所有函数（按地址排序）
    functions: Vec<FunctionInfo>,
    /// 名称 → 地址列表（一个名称可能对应多个同名函数）
    function_by_name: HashMap<String, Vec<u32>>,
    /// 所有全局变量
    variables: Vec<VariableInfo>,
    /// 名称 → 变量信息
    variable_by_name: HashMap<String, VariableInfo>,
}

impl DwarfResolver {
    /// 从 ELF 文件加载 DWARF 并构建解析器。
    pub fn from_elf(path: &Path) -> anyhow::Result<Self> {
        loader::with_dwarf(path, |dwarf| {
            let functions = collect_functions(dwarf)?;
            let variables = collect_variables(dwarf)?;
            Self::build(functions, variables)
        })
    }

    /// 从已提取的索引数据构建解析器（主要用于测试）。
    fn build(functions: Vec<FunctionInfo>, variables: Vec<VariableInfo>) -> anyhow::Result<Self> {
        let mut function_by_name: HashMap<String, Vec<u32>> = HashMap::new();
        for func in &functions {
            function_by_name
                .entry(func.name.clone())
                .or_default()
                .push(func.low_addr);
        }

        let mut variable_by_name: HashMap<String, VariableInfo> = HashMap::new();
        for var in &variables {
            variable_by_name.insert(var.name.clone(), var.clone());
        }

        Ok(DwarfResolver {
            functions,
            function_by_name,

            variables,
            variable_by_name,
        })
    }

    // ── 函数查询 ──

    /// 根据函数名查找入口地址。
    ///
    /// 同名多函数时返回第一个。遍历全部用 [`list_functions`]。
    pub fn function_addr(&self, name: &str) -> Option<u32> {
        self.function_by_name
            .get(name)
            .and_then(|addrs| addrs.first().copied())
    }

    /// 根据地址查找函数名和偏移。
    #[expect(dead_code)]
    pub fn addr_function(&self, addr: u32) -> Option<&str> {
        // 范围匹配：查找包含该地址的函数
        self.functions
            .iter()
            .find(|f| addr >= f.low_addr && addr < f.high_addr)
            .map(|f| f.name.as_str())
    }

    /// 列出所有函数。
    pub fn list_functions(&self) -> &[FunctionInfo] {
        &self.functions
    }

    /// 函数数量。
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    // ── 变量查询 ──

    /// 根据变量名查询全局变量信息。
    pub fn variable_info(&self, name: &str) -> Option<&VariableInfo> {
        self.variable_by_name.get(name)
    }

    /// 列出所有全局变量。
    pub fn list_variables(&self) -> &[VariableInfo] {
        &self.variables
    }

    /// 变量数量。
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    #[expect(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty() && self.variables.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空解析器应报告空。
    #[test]
    fn test_empty_resolver() {
        let resolver = DwarfResolver::build(vec![], vec![]).unwrap();
        assert!(resolver.is_empty());
        assert_eq!(resolver.function_count(), 0);
        assert_eq!(resolver.variable_count(), 0);
        assert!(resolver.function_addr("main").is_none());
        assert!(resolver.variable_info("x").is_none());
    }

    /// 单函数解析。
    #[test]
    fn test_single_function() {
        let funcs = vec![FunctionInfo {
            name: "main".into(),
            low_addr: 0x08000100,
            high_addr: 0x08000150,
        }];
        let resolver = DwarfResolver::build(funcs, vec![]).unwrap();
        assert_eq!(resolver.function_addr("main"), Some(0x08000100));
        assert_eq!(resolver.function_count(), 1);
        assert!(!resolver.is_empty());
    }

    /// 同名多函数应返回第一个。
    #[test]
    fn test_duplicate_function_names() {
        let funcs = vec![
            FunctionInfo {
                name: "reset".into(),
                low_addr: 0x08000000,
                high_addr: 0x08000010,
            },
            FunctionInfo {
                name: "reset".into(),
                low_addr: 0x08000100,
                high_addr: 0x08000110,
            },
        ];
        let resolver = DwarfResolver::build(funcs, vec![]).unwrap();
        // 返回第一个
        assert_eq!(resolver.function_addr("reset"), Some(0x08000000));
    }

    /// 地址在函数范围内应可查找。
    #[test]
    fn test_addr_to_function() {
        let funcs = vec![FunctionInfo {
            name: "main".into(),
            low_addr: 0x08000100,
            high_addr: 0x08000150,
        }];
        let resolver = DwarfResolver::build(funcs, vec![]).unwrap();
        // 精确匹配入口
        assert_eq!(resolver.addr_function(0x08000100), Some("main"));
        // 区间内
        assert_eq!(resolver.addr_function(0x08000120), Some("main"));
        // 区间外
        assert_eq!(resolver.addr_function(0x08000150), None); // high_addr 是 exclusive
    }

    /// 变量解析。
    #[test]
    fn test_variable_info() {
        let resolver = DwarfResolver::build(
            vec![],
            vec![VariableInfo {
                name: "adc_val".into(),
                addr: 0x20000010,
                size: 4,
                type_name: Some("uint32_t".into()),
            }],
        )
        .unwrap();
        let info = resolver.variable_info("adc_val").unwrap();
        assert_eq!(info.addr, 0x20000010);
        assert_eq!(info.size, 4);
        assert_eq!(info.type_name.as_deref(), Some("uint32_t"));
    }
}
