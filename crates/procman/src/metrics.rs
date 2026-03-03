use crate::table::ProcessTable;
use crate::process::ProcessStatus;
use std::fmt::Write;

pub fn get_metrics(table: &ProcessTable) -> String {
    let mut metrics_string = String::new();
    let all_processes = table.list();

    // Uptime
    let _ = writeln!(
        metrics_string,
        "# HELP aegis_pm_process_uptime_seconds Uptime of the process in seconds.\n# TYPE aegis_pm_process_uptime_seconds gauge"
    );
    for p in &all_processes {
        let _ = writeln!(
            metrics_string,
            "aegis_pm_process_uptime_seconds{{name=\"{}\"}} {}",
            p.name, p.uptime_seconds
        );
    }

    // Memory
    let _ = writeln!(
        metrics_string,
        "# HELP aegis_pm_process_memory_bytes Memory usage of the process in bytes.\n# TYPE aegis_pm_process_memory_bytes gauge"
    );
    for p in &all_processes {
        let _ = writeln!(
            metrics_string,
            "aegis_pm_process_memory_bytes{{name=\"{}\"}} {}",
            p.name, p.memory_bytes
        );
    }

    // CPU
    let _ = writeln!(
        metrics_string,
        "# HELP aegis_pm_process_cpu_percent CPU usage of the process.\n# TYPE aegis_pm_process_cpu_percent gauge"
    );
    for p in &all_processes {
        let _ = writeln!(
            metrics_string,
            "aegis_pm_process_cpu_percent{{name=\"{}\"}} {}",
            p.name, p.cpu_percent
        );
    }

    // Restarts
    let _ = writeln!(
        metrics_string,
        "# HELP aegis_pm_process_restarts_total Total number of times the process has been restarted.\n# TYPE aegis_pm_process_restarts_total counter"
    );
    for p in &all_processes {
        let _ = writeln!(
            metrics_string,
            "aegis_pm_process_restarts_total{{name=\"{}\"}} {}",
            p.name, p.restarts
        );
    }

    // Status Map (1 for Online, 0 for others)
    let _ = writeln!(
        metrics_string,
        "# HELP aegis_pm_process_status Whether the process is online (1) or not (0).\n# TYPE aegis_pm_process_status gauge"
    );
    for p in &all_processes {
        let status_val = if p.status == ProcessStatus::Online { 1 } else { 0 };
        let _ = writeln!(
            metrics_string,
            "aegis_pm_process_status{{name=\"{}\",status=\"{}\"}} {}",
            p.name, p.status, status_val
        );
    }

    metrics_string
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessInfo;

    #[test]
    fn test_prometheus_exposition_format() {
        let table = ProcessTable::new();
        let _ = table.add(ProcessInfo {
            name: "test-app".to_string(),
            pid: Some(1234),
            status: ProcessStatus::Online,
            restarts: 2,
            uptime_seconds: 3600,
            memory_bytes: 50_000_000,
            cpu_percent: 15.5,
        });

        let metrics = get_metrics(&table);
        
        assert!(metrics.contains("# HELP aegis_pm_process_uptime_seconds"));
        assert!(metrics.contains("aegis_pm_process_uptime_seconds{name=\"test-app\"} 3600"));
        assert!(metrics.contains("aegis_pm_process_memory_bytes{name=\"test-app\"} 50000000"));
        assert!(metrics.contains("aegis_pm_process_cpu_percent{name=\"test-app\"} 15.5"));
        assert!(metrics.contains("aegis_pm_process_restarts_total{name=\"test-app\"} 2"));
        assert!(metrics.contains("aegis_pm_process_status{name=\"test-app\",status=\"online\"} 1"));
    }
}
