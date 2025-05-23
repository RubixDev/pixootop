use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use amdgpu_sysfs::gpu_handle::GpuHandle;
use anyhow::{Context as _, Result};
use average::Averaged;
use chrono::{DateTime, Local};
use graceful::SignalGuard;
use libpulse_binding::volume::Volume;
use log::{error, info, trace};
use pulsectl::controllers::{DeviceControl as _, SinkController};
use reqwest::blocking::Client;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

mod average;

const SERVER_ADDR: &str = "http://192.168.178.40:6969";
const GPU_DEVICE_PATH: &str = "/sys/class/drm/card1/device";
const NETWORK_INTERFACE: &str = "enp37s0";
const MAX_NETWORK: f64 = 200_000.;

const PROGRESS_STEPS: u8 = 3;
const PROGRESS_RANGE: f64 = 15. * PROGRESS_STEPS as f64;

#[derive(Debug, serde::Serialize)]
pub struct Context {
    pub cpu: u8,
    pub mem: u8,
    pub gpu: u8,
    pub gpu_mem: u8,
    pub vol: u8,
    pub net_up: u8,
    pub net_down: u8,
    pub time: DateTime<Local>,
}

static STOP: AtomicBool = AtomicBool::new(false);

fn main() {
    env_logger::init();
    let signal_guard = SignalGuard::new();

    let handle = thread::spawn(main_loop);

    signal_guard.at_exit(|_| {
        info!("shutting down");
        STOP.store(true, Ordering::Release);
        if let Err(err) = handle.join().unwrap() {
            error!("{err:?}");
        }
    });
}

fn main_loop() -> Result<()> {
    let mut sys = System::new();
    let mut networks = Networks::new();
    let mut pulse = SinkController::create().context("creating sink controller")?;
    let gpu =
        GpuHandle::new_from_path(PathBuf::from(GPU_DEVICE_PATH)).context("getting gpu handle")?;
    let mut gpu_data = Averaged::<_, 25>::new(0u8);
    let mut net_up_data = Averaged::<_, 10>::new(0.);
    let mut net_down_data = Averaged::<_, 10>::new(0.);

    let client = Client::new();

    info!("initialization complete");
    while !STOP.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(100));

        sys.refresh_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let cpu = (sys
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage() as f64)
            .sum::<f64>()
            / sys.cpus().len() as f64
            / 100.
            * PROGRESS_RANGE)
            .round() as u8;

        let mem =
            (sys.used_memory() as f64 / sys.total_memory() as f64 * PROGRESS_RANGE).round() as u8;

        let gpu_mem_used = gpu.get_used_vram().context("reading GPU memory")?;
        let gpu_mem_total = gpu.get_total_vram().context("reading GPU total memory")?;
        let gpu_mem = (gpu_mem_used as f64 / gpu_mem_total as f64 * PROGRESS_RANGE).round() as u8;

        let gpu = gpu_data.next(gpu.get_busy_percent().context("reading GPU usage")?, 100.);

        let dev = pulse.get_default_device().context("getting pulse device")?;
        let avg = dev.volume.avg().0;
        let base_delta = (Volume::NORMAL.0 as f64 - Volume::MUTED.0 as f64) / PROGRESS_RANGE;
        let vol = ((avg - Volume::MUTED.0) as f64 / base_delta).round() as u8;

        networks.refresh(true);
        let net = networks
            .get(NETWORK_INTERFACE)
            .context("get network interface")?;
        let net_up = net_up_data.next(net.transmitted() as f64, MAX_NETWORK);
        let net_down = net_down_data.next(net.received() as f64, MAX_NETWORK);

        let ctx = Context {
            cpu,
            mem,
            gpu,
            gpu_mem,
            vol,
            net_up,
            net_down,
            time: Local::now(),
        };
        trace!("updated context: {ctx:?}");

        if let Err(err) = client
            .post(format!("{SERVER_ADDR}/state"))
            .json(&ctx)
            .send()
        {
            error!("error while sending request: {err:?}");
        }
    }
    client
        .post(format!("{SERVER_ADDR}/reset-state"))
        .send()
        .context("resetting state")?;
    Ok(())
}
