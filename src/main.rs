use std::{panic, path::PathBuf, str::FromStr, sync::Arc, thread, time::Duration};

use amdgpu_sysfs::gpu_handle::GpuHandle;
use ansi_to_tui::IntoText;
use anyhow::{Context as _, Result};
use average::Averaged;
use bluetooth_serial_port::BtAddr;
use chrono::Local;
use crossbeam::{
    channel::{self, Receiver},
    queue::ArrayQueue,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
};
use image::DynamicImage;
use libpulse_binding::volume::Volume;
use log::{debug, error, info, trace};
use pixoo::{
    mode::{LightEffectMode, LightMode},
    Brightness, Pixoo, DISPLAY_SIZE,
};
use pulsectl::controllers::{DeviceControl, SinkController};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Padding, Paragraph},
    DefaultTerminal,
};
use render::Context;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};
use tui_logger::{LevelFilter, TuiLoggerSmartWidget, TuiWidgetEvent, TuiWidgetState};

mod average;
mod fonts;
mod render;

fn main() {
    tui_logger::init_logger(LevelFilter::Trace)
        .context("initializing logger")
        .unwrap_or_else(|e| panic!("{e:?}"));
    tui_logger::set_default_level(LevelFilter::Trace);

    debug!("logger initialized");

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture).expect("failed enabling mouse events");
    while let Err(err) = try_main(&mut terminal) {
        error!("encountered error: {err:?}");
        thread::sleep(Duration::from_secs(1));
    }
    info!("graceful shutdown...");
    execute!(std::io::stdout(), DisableMouseCapture).expect("failed disabling mouse events");
    ratatui::restore();
}

const PIXOO_MAC_ADDR: &str = "11:75:58:35:2B:35";
const GPU_DEVICE_PATH: &str = "/sys/class/drm/card1/device";
const NETWORK_INTERFACE: &str = "enp37s0";
const MAX_NETWORK: f64 = 200_000.;

const PROGRESS_STEPS: u8 = 3;
const PROGRESS_RANGE: f64 = 15. * PROGRESS_STEPS as f64;

fn try_main(terminal: &mut DefaultTerminal) -> Result<()> {
    let log_state = TuiWidgetState::new()
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
    let (s, r) = channel::unbounded();
    let r2 = r.clone();
    let mut jh = thread::spawn(|| pixoo_worker(rx, r2));

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
        let img = DynamicImage::from(render::create_frame(ctx));
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
        tx.force_push(Message::Frame(img));

        if jh.is_finished() {
            match jh.join() {
                Ok(res) => {
                    if let Err(err) = res {
                        error!("pixoo worker encountered error: {err:?}");
                    }
                    let rx = Arc::clone(&tx);
                    let r = r.clone();
                    jh = thread::spawn(|| pixoo_worker(rx, r));
                }
                Err(e) => panic::resume_unwind(e),
            }
        }
        if event::poll(Duration::from_millis(100)).context("event poll failed")? {
            match event::read().context("event read failed")? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => {
                        tx.force_push(Message::Quit);
                        if let Err(e) = jh.join() {
                            panic::resume_unwind(e);
                        }
                        return Ok(());
                    }
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
                },
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollDown => _ = s.send(QueuedMessage::BrightnessDown),
                        MouseEventKind::ScrollUp => _ = s.send(QueuedMessage::BrightnessUp),
                        _ => {}
                    };
                }
                _ => {}
            }
        }
    }
}

enum Message {
    Frame(DynamicImage),
    Quit,
}

enum QueuedMessage {
    BrightnessUp,
    BrightnessDown,
}

fn pixoo_worker(rx: Arc<ArrayQueue<Message>>, r: Receiver<QueuedMessage>) -> Result<()> {
    let mut pixoo =
        Pixoo::connect(BtAddr::from_str(PIXOO_MAC_ADDR).unwrap()).context("connecting to pixoo")?;
    debug!("connected to Pixoo");
    let mut brightness = Brightness::new(30).unwrap();
    pixoo
        .set_brightness(brightness)
        .context("setting brightness")?;

    loop {
        thread::sleep(Duration::from_millis(100));
        match rx.pop() {
            Some(Message::Frame(img)) => {
                trace!("sending new image to device");
                pixoo.set_image(&img).context("sending frame")?;
            }
            Some(Message::Quit) => {
                info!("turning display off");
                pixoo
                    .set_mode(LightMode::Light {
                        color: [255, 0, 255],
                        brightness,
                        effect_mode: LightEffectMode::Normal,
                        on: false,
                    })
                    .context("turning display off")?;
                return Ok(());
            }
            None => {}
        }
        match r.try_recv() {
            Ok(msg @ (QueuedMessage::BrightnessUp | QueuedMessage::BrightnessDown)) => {
                match msg {
                    QueuedMessage::BrightnessUp => brightness = brightness.saturating_add(5),
                    _ => brightness = brightness.saturating_sub(5),
                }
                debug!("setting brightness to {brightness}");
                pixoo
                    .set_brightness(brightness)
                    .context("setting brightness")?;
            }
            Err(_) => {}
        }
    }
}
