use std::sync::atomic::{AtomicUsize, Ordering};

use camera::discover_all_cameras;
use unros_core::{
    anyhow::{self, Context},
    async_run_all, default_run_options,
    log::info,
    tokio, signal::Subscriber,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let fps = 20;
    let cameras = discover_all_cameras()
        .context("Failed to discover cameras")?
        .map(|mut camera| {
            let mut sub = Subscriber::default();
            camera.accept_image_received_sub(sub.create_subscription(8));
            let frame_count = AtomicUsize::new(0);
            let idx = camera.camera_index;
            tokio::spawn(async move {
                loop {
                    sub.recv().await;
                    let frame_count = frame_count.fetch_add(1, Ordering::Relaxed) + 1;
                    if frame_count % (fps as usize) == 0 {
                        info!("idx: {idx} -> {frame_count}");
                    }
                }
            });
            camera.into()
        });

    async_run_all(cameras, default_run_options!()).await
}
