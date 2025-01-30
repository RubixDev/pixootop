use std::{panic, path::PathBuf, str::FromStr, sync::Arc, thread, time::Duration};

use amdgpu_sysfs::gpu_handle::GpuHandle;
use ansi_to_tui::IntoText;
use anyhow::{Context as _, Result};
use average::Averaged;
use bluetooth_serial_port::BtAddr;
use chrono::{DateTime, Local};
use crossbeam::queue::ArrayQueue;
use crossterm::event::{self, Event, KeyCode};
use image::{DynamicImage, GenericImage, Rgb, RgbImage};
use libpulse_binding::volume::Volume;
use log::{debug, error, info, trace};
use pixoo::{Brightness, Pixoo, DISPLAY_SIZE};
use pulsectl::controllers::{DeviceControl, SinkController};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Padding, Paragraph},
    DefaultTerminal,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};
use tui_logger::{LevelFilter, TuiLoggerSmartWidget, TuiWidgetEvent, TuiWidgetState};

mod average;
mod fonts;

fn main() {
    tui_logger::init_logger(LevelFilter::Trace)
        .context("initializing logger")
        .unwrap_or_else(|e| panic!("{e:?}"));
    tui_logger::set_default_level(LevelFilter::Trace);

    debug!("logger initialized");

    let mut terminal = ratatui::init();
    while let Err(err) = try_main(&mut terminal) {
        error!("encountered error: {err:?}");
        thread::sleep(Duration::from_secs(1));
    }
    info!("graceful shutdown...");
    ratatui::restore();
}

const PIXOO_MAC_ADDR: &str = "11:75:58:35:2B:35";
const GPU_DEVICE_PATH: &str = "/sys/class/drm/card1/device";
const NETWORK_INTERFACE: &str = "enp37s0";

const PROGRESS_STEPS: u8 = 3;
const PROGRESS_RANGE: f64 = 15. * PROGRESS_STEPS as f64;

fn try_main(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut log_state = TuiWidgetState::new()
        .set_default_display_level(LevelFilter::Info)
        .set_level_for_target("pixootop", LevelFilter::Debug);
    log_state.transition(TuiWidgetEvent::HideKey);

    let layout = Layout::vertical([
        Constraint::Min(DISPLAY_SIZE as u16 / 2 + 2),
        Constraint::Percentage(100),
    ]);

    info!("app initialized");

    let tx = Arc::new(ArrayQueue::new(1));
    let rx = Arc::clone(&tx);
    let mut jh = thread::spawn(|| pixoo_worker(rx));

    let mut sys = System::new();
    let mut networks = Networks::new();
    // TODO: try to keep sink controller between main loops
    let mut pulse = SinkController::create().context("creating sink controller")?;
    let gpu =
        GpuHandle::new_from_path(PathBuf::from(GPU_DEVICE_PATH)).context("getting gpu handle")?;
    let mut gpu_data = Averaged::<_, 25>::new(0u8);
    let mut net_up_data = Averaged::<_, 10>::new(0.);
    let mut net_down_data = Averaged::<_, 10>::new(0.);
    loop {
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
        let net_up = net_up_data.next(net.transmitted() as f64, 200_000.);
        let net_down = net_down_data.next(net.received() as f64, 200_000.);

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
        let img = DynamicImage::from(create_frame(ctx));
        let ansi_img = ansipix::of_image(
            &img,
            (DISPLAY_SIZE as usize, DISPLAY_SIZE as usize),
            0,
            false,
        )
        .into_text()
        .context("parsing ANSI")?;
        terminal
            .draw(|frame| {
                let layout = layout.split(frame.area());
                frame.render_widget(
                    Paragraph::new(ansi_img).block(
                        Block::bordered()
                            .title("Display")
                            .padding(Padding::horizontal(1)),
                    ),
                    layout[0],
                );
                frame.render_widget(
                    TuiLoggerSmartWidget::default()
                        .border_type(BorderType::Plain)
                        .title_log("Logs")
                        .title_target("Log Filter")
                        .style_error(Style::default().fg(Color::Red))
                        .style_warn(Style::default().fg(Color::Yellow))
                        .style_info(Style::default().fg(Color::Green))
                        .style_debug(Style::default().fg(Color::Cyan))
                        .style_trace(Style::default().fg(Color::Magenta))
                        .output_file(false)
                        .output_line(false)
                        .output_separator(' ')
                        .state(&log_state),
                    layout[1],
                );
            })
            .context("drawing terminal")?;
        tx.force_push(img);

        if jh.is_finished() {
            match jh.join() {
                Ok(res) => {
                    if let Err(err) = res {
                        error!("pixoo worker encountered error: {err:?}");
                    }
                    let rx = Arc::clone(&tx);
                    jh = thread::spawn(|| pixoo_worker(rx));
                }
                Err(e) => panic::resume_unwind(e),
            }
        }
        if event::poll(Duration::from_millis(100)).context("event poll failed")? {
            if let Event::Key(key) = event::read().context("event read failed")? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(' ') => log_state.transition(TuiWidgetEvent::SpaceKey),
                    KeyCode::Esc => log_state.transition(TuiWidgetEvent::EscapeKey),
                    KeyCode::PageUp => log_state.transition(TuiWidgetEvent::PrevPageKey),
                    KeyCode::PageDown => log_state.transition(TuiWidgetEvent::NextPageKey),
                    KeyCode::Up => log_state.transition(TuiWidgetEvent::UpKey),
                    KeyCode::Down => log_state.transition(TuiWidgetEvent::DownKey),
                    KeyCode::Left => log_state.transition(TuiWidgetEvent::LeftKey),
                    KeyCode::Right => log_state.transition(TuiWidgetEvent::RightKey),
                    KeyCode::Char('+') => log_state.transition(TuiWidgetEvent::PlusKey),
                    KeyCode::Char('-') => log_state.transition(TuiWidgetEvent::MinusKey),
                    KeyCode::Char('h') => log_state.transition(TuiWidgetEvent::HideKey),
                    KeyCode::Char('f') => log_state.transition(TuiWidgetEvent::FocusKey),
                    _ => {}
                }
            }
        }
    }
}

fn pixoo_worker(rx: Arc<ArrayQueue<DynamicImage>>) -> Result<()> {
    let mut pixoo =
        Pixoo::connect(BtAddr::from_str(PIXOO_MAC_ADDR).unwrap()).context("connecting to pixoo")?;
    debug!("connected to Pixoo");
    pixoo.set_brightness(Brightness::new(30).unwrap()).unwrap();

    loop {
        thread::sleep(Duration::from_millis(100));
        if let Some(img) = rx.pop() {
            trace!("sending new image to device");
            pixoo.set_image(&img).context("sending frame")?;
        }
    }
}

#[derive(Debug)]
struct Context {
    cpu: u8,
    mem: u8,
    gpu: u8,
    gpu_mem: u8,
    vol: u8,
    net_up: u8,
    net_down: u8,
    time: DateTime<Local>,
}

fn create_frame(ctx: Context) -> RgbImage {
    let mut img = RgbImage::new(DISPLAY_SIZE, DISPLAY_SIZE);

    draw_progress(
        ctx.vol,
        &mut img,
        0,
        [
            Rgb([0xff, 0xaa, 0x00]),
            Rgb([0xbb, 0x88, 0x00]),
            Rgb([0x88, 0x55, 0x00]),
        ],
    );
    draw_progress(
        ctx.mem,
        &mut img,
        1,
        [
            Rgb([0x00, 0xdd, 0x00]),
            Rgb([0x00, 0x99, 0x00]),
            Rgb([0x00, 0x55, 0x00]),
        ],
    );
    draw_progress(
        ctx.cpu,
        &mut img,
        2,
        [
            Rgb([0x00, 0x00, 0xdd]),
            Rgb([0x00, 0x00, 0x99]),
            Rgb([0x00, 0x00, 0x55]),
        ],
    );
    draw_progress(
        ctx.gpu,
        &mut img,
        3,
        [
            Rgb([0xff, 0x77, 0x00]),
            Rgb([0xbb, 0x55, 0x00]),
            Rgb([0x88, 0x33, 0x00]),
        ],
    );
    draw_progress(
        ctx.gpu_mem,
        &mut img,
        4,
        [
            Rgb([0xdd, 0x00, 0x00]),
            Rgb([0x99, 0x00, 0x00]),
            Rgb([0x55, 0x00, 0x00]),
        ],
    );
    draw_progress(
        ctx.net_up,
        &mut img,
        5,
        [
            Rgb([0x00, 0x99, 0xff]),
            Rgb([0x00, 0x77, 0xbb]),
            Rgb([0x00, 0x55, 0x88]),
        ],
    );
    draw_progress(
        ctx.net_down,
        &mut img,
        6,
        [
            Rgb([0x99, 0x00, 0xff]),
            Rgb([0x66, 0x00, 0x99]),
            Rgb([0x44, 0x00, 0x66]),
        ],
    );

    write_string(
        &ctx.time.format("%l:%M").to_string(),
        &mut img,
        (3, 7),
        &[Rgb([0xff, 0x00, 0xff]), Rgb([0xff, 0x00, 0x99])],
        &fonts::FONT_3X5,
        false,
    );
    write_string(
        &ctx.time.format("%S").to_string(),
        &mut img,
        (10, 12),
        &[Rgb([0x88, 0x00, 0x88]), Rgb([0x88, 0x00, 0x55])],
        &fonts::FONT_3X4,
        false,
    );

    img
}

fn draw_progress<I: GenericImage>(progress: u8, img: &mut I, y: u32, colors: [I::Pixel; 3]) {
    let full = (progress / PROGRESS_STEPS).min(15);
    let rest = match full >= 15 {
        true => 0,
        false => progress % PROGRESS_STEPS,
    };
    for x in 0..=full {
        img.put_pixel(x as u32, y, colors[0]);
    }
    match rest {
        1 => img.put_pixel(full as u32 + 1, y, colors[2]),
        2 => img.put_pixel(full as u32 + 1, y, colors[1]),
        _ => {}
    }
}

fn write_string<I: GenericImage>(
    str: &str,
    img: &mut I,
    (mut x, y): (u32, u32),
    colors: &[I::Pixel],
    font: &phf::Map<char, &[&[bool]]>,
    spaces: bool,
) {
    let mut i = 0;
    for char in str.chars() {
        let Some(&char) = font.get(&char) else {
            continue;
        };

        for (cy, &line) in char.iter().enumerate() {
            for (cx, &px) in line.iter().enumerate() {
                if px {
                    img.put_pixel(x + cx as u32, y + cy as u32, colors[i]);
                }
            }
        }

        x += char[0].len() as u32 + spaces as u32;
        i += 1;
        i %= colors.len();
    }
}
