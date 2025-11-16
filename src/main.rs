// src/main.rs
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use sysinfo::{CpuExt, System, SystemExt, NetworkExt, NetworksExt, ProcessExt, ProcessStatus};

#[derive(Debug, Serialize, Deserialize)]
struct SystemStats {
    cpu: Vec<f32>,
    mem: MemoryStats,
    swap: MemoryStats,
    net: std::collections::HashMap<String, NetworkStats>,
    proc: ProcessStats,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryStats {
    total: u64,
    used: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct NetworkStats {
    rx: u64,
    tx: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessStats {
    total: usize,
    running: usize,
    sleeping: usize,
    zombie: usize,
}

struct ResourceMonitor {
    system: System,
    last_net_data: std::collections::HashMap<String, (u64, u64)>,
    last_update: Instant,
}

impl ResourceMonitor {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let last_net_data = Self::get_network_data(&system);

        Self {
            system,
            last_net_data,
            last_update: Instant::now(),
        }
    }

    fn get_network_data(system: &System) -> std::collections::HashMap<String, (u64, u64)> {
        let mut net_data = std::collections::HashMap::new();
        for (interface_name, data) in system.networks() {
            net_data.insert(
                interface_name.clone(),
                (data.total_received(), data.total_transmitted())
            );
        }
        net_data
    }

    fn update(&mut self) -> SystemStats {
        self.system.refresh_all();

        let cpu_usage: Vec<f32> = self.system.cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect();

        let mem = MemoryStats {
            total: self.system.total_memory(),
            used: self.system.used_memory(),
        };

        let swap = MemoryStats {
            total: self.system.total_swap(),
            used: self.system.used_swap(),
        };

        let current_net_data = Self::get_network_data(&self.system);
        let elapsed = self.last_update.elapsed().as_secs_f64();

        let mut net = std::collections::HashMap::new();
        for (interface, &(current_rx, current_tx)) in &current_net_data {
            if let Some(&(last_rx, last_tx)) = self.last_net_data.get(interface) {
                let rx_rate = ((current_rx - last_rx) as f64 / elapsed) as u64;
                let tx_rate = ((current_tx - last_tx) as f64 / elapsed) as u64;

                net.insert(interface.clone(), NetworkStats {
                    rx: rx_rate,
                    tx: tx_rate,
                });
            }
        }

        self.last_net_data = current_net_data;
        self.last_update = Instant::now();

        let mut running = 0;
        let mut sleeping = 0;
        let mut zombie = 0;

        for process in self.system.processes().values() {
            match process.status() {
                ProcessStatus::Run => running += 1,
                ProcessStatus::Sleep => sleeping += 1,
                ProcessStatus::Zombie => zombie += 1,
                _ => {},
            }
        }

        let proc = ProcessStats {
            total: self.system.processes().len(),
            running,
            sleeping,
            zombie,
        };

        SystemStats {
            cpu: cpu_usage,
            mem,
            swap,
            net,
            proc,
        }
    }
}

async fn send_stats(stats: &SystemStats, endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let response = client
        .post(endpoint)
        .json(stats)
        .send()
        .await?;

    if response.status().is_success() {
        println!("数据发送成功");
    } else {
        eprintln!("发送失败: {}", response.status());
    }

    Ok(())
}

fn display_stats(stats: &SystemStats) {
    println!("\x1B[2J\x1B[1;1H"); 
    println!("=== 系统资源监控 ===");

    println!("CPU核心数: {}", stats.cpu.len());
    for (i, usage) in stats.cpu.iter().enumerate() {
        println!("  核心 {}: {:.1}%", i, usage);
    }
    let avg_cpu: f32 = stats.cpu.iter().sum::<f32>() / stats.cpu.len() as f32;
    println!("平均CPU使用率: {:.1}%", avg_cpu);

    println!("内存: {} / {} ({:.1}%)",
             format_bytes(stats.mem.used),
             format_bytes(stats.mem.total),
             (stats.mem.used as f64 / stats.mem.total as f64) * 100.0
    );

    println!("交换空间: {} / {} ({:.1}%)",
             format_bytes(stats.swap.used),
             format_bytes(stats.swap.total),
             (stats.swap.used as f64 / stats.swap.total as f64) * 100.0
    );

    println!("网络接口:");
    for (interface, net_stats) in &stats.net {
        println!("  {}: 接收 {}/s, 发送 {}/s",
                 interface,
                 format_bytes(net_stats.rx),
                 format_bytes(net_stats.tx)
        );
    }

    println!("进程统计:");
    println!("  总计: {}, 运行: {}, 睡眠: {}, 僵尸: {}",
             stats.proc.total, stats.proc.running, stats.proc.sleeping, stats.proc.zombie
    );

    println!("\n数据已发送到 http://localhost:25800");
    println!("按 Ctrl+C 退出");
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("System Monitor")
        .version("1.0")
        .author("Your Name")
        .about("监控Linux系统资源使用情况并发送JSON数据")
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .value_name("SECONDS")
                .help("刷新间隔(秒)")
                .default_value("1")
        )
        .arg(
            Arg::new("endpoint")
                .short('e')
                .long("endpoint")
                .value_name("URL")
                .help("接收数据的端点URL")
                .default_value("http://localhost:25800")
        )
        .arg(
            Arg::new("no-display")
                .long("no-display")
                .help("不显示监控信息，只发送数据")
        )
        .get_matches();

    let interval_secs: u64 = matches.get_one::<String>("interval")
        .unwrap()
        .parse()
        .unwrap_or(1);
    let endpoint = matches.get_one::<String>("endpoint").unwrap();
    let no_display = matches.contains_id("no-display");

    let mut monitor = ResourceMonitor::new();

    println!("开始监控系统资源...");
    println!("刷新间隔: {} 秒", interval_secs);
    println!("数据端点: {}", endpoint);
    println!("按 Ctrl+C 退出\n");

    tokio::time::sleep(Duration::from_secs(2)).await;

    loop {
        let stats = monitor.update();

        if let Err(e) = send_stats(&stats, endpoint).await {
            eprintln!("发送数据失败: {}", e);
        }

        if !no_display {
            display_stats(&stats);
        }

        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}