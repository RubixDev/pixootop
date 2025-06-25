use std::{
    str::FromStr as _,
    sync::atomic::{AtomicU8, Ordering},
    time::Duration,
};

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, post,
    web::{Data, Json, Path},
};
use anyhow::{Context as _, Result};
use bluetooth_serial_port::BtAddr;
use chrono::Local;
use image::DynamicImage;
use log::{debug, error, info, trace};
use pixoo::{
    Brightness, Pixoo,
    mode::{LightEffectMode, LightMode},
};
use render::Context;
use tokio::{
    select,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::{self, Instant},
};
use tokio_util::sync::CancellationToken;

mod fonts;
mod render;

const PIXOO_MAC_ADDR: &str = "11:75:58:35:2B:35";
const PROGRESS_STEPS: u8 = 3;

static BRIGHTNESS: AtomicU8 = AtomicU8::new(30);

#[derive(PartialEq)]
enum Message {
    Frame(DynamicImage),
    On,
    Off,
    BrightnessUp,
    BrightnessDown,
    Brightness(Brightness),
}

async fn pixoo_loop(rx: &mut UnboundedReceiver<Message>) -> Result<()> {
    let mut pixoo =
        Pixoo::connect(BtAddr::from_str(PIXOO_MAC_ADDR).unwrap()).context("connecting to pixoo")?;
    debug!("connected to Pixoo");
    pixoo
        .set_brightness(Brightness::new_saturating(
            BRIGHTNESS.load(Ordering::SeqCst),
        ))
        .context("setting brightness")?;
    let mut on = true;

    loop {
        time::sleep(Duration::from_millis(50)).await;
        while let Ok(msg) = rx.try_recv() {
            if !on && msg != Message::On {
                continue;
            }
            match msg {
                Message::Frame(img) => {
                    trace!("sending new image to device");
                    pixoo.set_image(&img).context("sending frame")?;
                }
                Message::On => {
                    info!("turning display on");
                    on = true;
                }
                Message::Off => {
                    info!("turning display off");
                    on = false;
                    pixoo
                        .set_mode(LightMode::Light {
                            color: [255, 0, 255],
                            brightness: Brightness::new_saturating(
                                BRIGHTNESS.load(Ordering::SeqCst),
                            ),
                            effect_mode: LightEffectMode::Normal,
                            on: false,
                        })
                        .context("turning display off")?;
                }
                Message::BrightnessUp => {
                    let brightness = Brightness::new_saturating(
                        BRIGHTNESS
                            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |b| {
                                Some(b.saturating_add(5))
                            })
                            .unwrap()
                            .saturating_add(5),
                    );
                    debug!("setting brightness to {brightness}");
                    pixoo
                        .set_brightness(brightness)
                        .context("setting brightness")?;
                }
                Message::BrightnessDown => {
                    let brightness = Brightness::new_saturating(
                        BRIGHTNESS
                            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |b| {
                                Some(b.saturating_sub(5))
                            })
                            .unwrap()
                            .saturating_sub(5),
                    );
                    debug!("setting brightness to {brightness}");
                    pixoo
                        .set_brightness(brightness)
                        .context("setting brightness")?;
                }
                Message::Brightness(brightness) => {
                    BRIGHTNESS.store(brightness.into(), Ordering::SeqCst);
                    debug!("setting brightness to {brightness}");
                    pixoo
                        .set_brightness(brightness)
                        .context("setting brightness")?;
                }
            }
        }
    }
}

async fn render_loop(mut rx: UnboundedReceiver<Option<Context>>, tx: UnboundedSender<Message>) {
    let mut state = None;
    let mut last_state_update = Instant::now();
    loop {
        time::sleep(Duration::from_millis(100)).await;
        while let Ok(ctx) = rx.try_recv() {
            state = ctx;
            last_state_update = Instant::now();
        }

        if last_state_update.elapsed() >= Duration::from_secs(60) {
            if state.is_some() {
                info!("client disconnected");
            }
            state = None;
        }

        let img = DynamicImage::from(render::create_frame(
            state,
            Local::now(),
            last_state_update.elapsed() >= Duration::from_secs(2),
        ));
        _ = tx.send(Message::Frame(img));
    }
}

type AppData = Data<(UnboundedSender<Message>, UnboundedSender<Option<Context>>)>;

#[post("/off")]
async fn turn_off(data: AppData) -> impl Responder {
    _ = data.0.send(Message::Off);
    HttpResponse::Ok()
}

#[post("/on")]
async fn turn_on(data: AppData) -> impl Responder {
    _ = data.0.send(Message::On);
    HttpResponse::Ok()
}

#[post("/brightness-up")]
async fn brightness_up(data: AppData) -> impl Responder {
    _ = data.0.send(Message::BrightnessUp);
    HttpResponse::Ok()
}

#[post("/brightness-down")]
async fn brightness_down(data: AppData) -> impl Responder {
    _ = data.0.send(Message::BrightnessDown);
    HttpResponse::Ok()
}

#[post("/brightness/{brightness}")]
async fn set_brightness(data: AppData, path: Path<Brightness>) -> impl Responder {
    _ = data.0.send(Message::Brightness(path.into_inner()));
    HttpResponse::Ok()
}

#[get("/brightness")]
async fn get_brightness() -> impl Responder {
    HttpResponse::Ok().body(BRIGHTNESS.load(Ordering::SeqCst).to_string())
}

#[post("/state")]
async fn set_state(data: AppData, Json(body): Json<Context>) -> impl Responder {
    _ = data.1.send(Some(body));
    HttpResponse::Ok()
}

#[post("/reset-state")]
async fn reset_state(data: AppData) -> impl Responder {
    _ = data.1.send(None);
    HttpResponse::Ok()
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body(include_str!("./index.html"))
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cancel = CancellationToken::new();

    let (pixoo_tx, mut pixoo_rx) = mpsc::unbounded_channel();
    let cancel_clone = cancel.clone();
    let pixoo_job = tokio::spawn(async move {
        while let Err(err) = pixoo_loop(&mut pixoo_rx).await {
            error!("pixoo service encountered error: {err:?}");
            if cancel_clone.is_cancelled() {
                return;
            }
        }
    });

    let (state_tx, state_rx) = mpsc::unbounded_channel();
    let pixoo_tx_2 = pixoo_tx.clone();
    let cancel_clone = cancel.clone();
    let render_job = tokio::spawn(async move {
        select! {
            _ = render_loop(state_rx, pixoo_tx_2) => {}
            _ = cancel_clone.cancelled() => {}
        }
    });

    let data = Data::new((pixoo_tx, state_tx));
    HttpServer::new(move || {
        App::new()
            .app_data(Data::clone(&data))
            .service(turn_off)
            .service(turn_on)
            .service(brightness_up)
            .service(brightness_down)
            .service(set_brightness)
            .service(get_brightness)
            .service(set_state)
            .service(reset_state)
            .service(index)
    })
    .bind(("0.0.0.0", 6969))?
    .run()
    .await?;

    _ = tokio::signal::ctrl_c().await;
    cancel.cancel();
    pixoo_job.await.unwrap();
    render_job.await.unwrap();

    Ok(())
}
