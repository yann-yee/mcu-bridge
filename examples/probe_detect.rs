/// 探针检测样例 — 列出所有已连接的调试探针。
use probe_rs::probe::list::Lister;

fn main() {
    println!("=== probe-rs 探针检测 ===");
    let lister = Lister::new();
    let probes = lister.list_all();
    println!("找到 {} 个探针:", probes.len());
    for (i, p) in probes.iter().enumerate() {
        println!(
            "  [{i}] {} (vid={:04x} pid={:04x})",
            p.identifier, p.vendor_id, p.product_id,
        );
        if let Some(serial) = &p.serial_number {
            println!("      序列号: {serial}");
        }
        println!("      接口: {:?}", p.interface);
    }
    if probes.is_empty() {
        println!("未检测到任何探针。请确认 USB 连接和驱动。");
    }
}
