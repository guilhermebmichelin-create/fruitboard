//! Shared opt-in probe utilities; never linked into the application runtime.
//! Windows counters are process-lifetime peaks, including fixture/setup work.

#[repr(C)]
#[derive(Default)]
struct Counters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_usage: usize,
}

// PROCESS_MEMORY_COUNTERS_EX and the Windows 7+ kernel32 API. No new dependency
// or production capability is needed. Query only this probe or its owned child.
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    fn K32GetProcessMemoryInfo(
        handle: *mut std::ffi::c_void,
        counters: *mut Counters,
        bytes: u32,
    ) -> i32;
}

pub fn memory(owned_child: Option<u32>) -> Result<serde_json::Value, &'static str> {
    // A PID here comes only from the supervisor's still-owned Child. No process
    // enumeration, arbitrary PID input, mutation or termination is performed.
    let handle = unsafe {
        owned_child.map_or_else(|| GetCurrentProcess(), |pid| OpenProcess(0x410, 0, pid))
    };
    if handle.is_null() {
        return Err("owned_process_query_failed");
    }
    let mut counters = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        ..Counters::default()
    };
    let bytes = counters.cb;
    let ok = unsafe { K32GetProcessMemoryInfo(handle, &mut counters, bytes) };
    if owned_child.is_some() {
        unsafe { CloseHandle(handle) };
    }
    if ok == 0 {
        return Err("owned_process_memory_failed");
    }
    Ok(serde_json::json!({
        "workingSetBytes": counters.working_set_size,
        "privateCommitBytes": counters.private_usage,
        "lifetimePeakWorkingSetBytes": counters.peak_working_set_size,
        "lifetimePeakCommitBytes": counters.peak_pagefile_usage
    }))
}

pub fn warm_summary(samples: &[f64]) -> serde_json::Value {
    assert_eq!(samples.len(), 10, "ten measured samples required");
    assert!(samples.iter().all(|v| v.is_finite() && *v >= 0.0));
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    serde_json::json!({
        "samples":samples, "medianMs":(sorted[4]+sorted[5])/2.0,
        "nearestRankP95Ms":sorted[9], "maxMs":sorted[9]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_keeps_raw_order_and_uses_nearest_rank_p95_for_ten_runs() {
        let samples = [9.0, 1.0, 3.0, 7.0, 4.0, 2.0, 5.0, 6.0, 10.0, 8.0];
        let report = warm_summary(&samples);
        assert_eq!(report["samples"], serde_json::json!(samples));
        assert_eq!(report["medianMs"], 5.5);
        assert_eq!(report["nearestRankP95Ms"], 10.0);
        assert_eq!(report["maxMs"], 10.0);
    }

    #[test]
    #[should_panic]
    fn summary_refuses_nonfinite_samples() {
        warm_summary(&[f64::NAN; 10]);
    }
}
