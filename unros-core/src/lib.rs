use std::{future::Future, pin::Pin, sync::Arc, time::Instant};

pub use anyhow;
pub use async_trait::async_trait;
pub use log;
use log::{error, info};
use serde::Deserialize;
use static_assertions::assert_impl_all;
pub use tokio;
use tokio::task::JoinSet;
pub use tokio_rayon::{self, rayon};

// pub trait Variadic {
//     fn contains<T: 'static>() -> bool;
//     fn is_unique<T>() -> bool;
//     fn len() -> usize;
// }

// impl Variadic for () {
//     fn contains<T>() -> bool {
//         false
//     }
//     fn is_unique<T>() -> bool {
//         true
//     }
//     fn len() -> usize {
//         0
//     }
// }

// impl<A: 'static, X: Variadic> Variadic for (A, X) {
//     fn contains<T: 'static>() -> bool {
//         if TypeId::of::<A>() == TypeId::of::<T>() {
//             true
//         } else {
//             X::contains::<T>()
//         }
//     }
//     fn is_unique<T>() -> bool {
//         X::contains::<A>()
//     }
//     fn len() -> usize {
//         1 + X::len()
//     }
// }

#[macro_export]
macro_rules! node_info {
    ($node: expr, $($arg:tt)+) => {
        $crate::log::info!(target: $node.get_name(), $($arg)+)
    };
}

#[macro_export]
macro_rules! node_warn {
    ($node: expr, $($arg:tt)+) => {
        $crate::log::warn!(target: $node.get_name(), $($arg)+)
    };
}

#[macro_export]
macro_rules! node_error {
    ($node: expr, $($arg:tt)+) => {
        $crate::log::error!(target: $node.get_name(), $($arg)+)
    };
}

// pub struct RuntimeContext {
//     alive_receiver: broadcast::Receiver<()>,
// }

// impl Clone for RuntimeContext {
//     fn clone(&self) -> Self {
//         Self { alive_receiver: self.alive_receiver.resubscribe() }
//     }
// }

// impl RuntimeContext {
//     pub fn is_alive(&self) -> bool {
//         matches!( self.alive_receiver.try_recv(), Err(TryRecvError::Empty) | Err(TryRecvError::Lagged(_)) | Ok(()))
//     }

//     pub async fn wait_until_death(&mut self) {
//         loop {
//             if let Err(RecvError::Closed) = self.alive_receiver.recv().await {
//                 break
//             }
//         }
//     }
// }

#[async_trait]
pub trait Node: Send + 'static {
    fn set_name(&mut self, name: String);
    fn get_name(&self) -> &str;
    async fn run(self) -> anyhow::Result<()>;
}

pub struct Signal<T: Clone> {
    async_fns: Vec<Box<dyn Fn(T) -> Box<dyn Future<Output = ()> + Send + Unpin> + Send + Sync>>,
    fns: Vec<Box<dyn Fn(T) + Send + Sync>>,
}

impl<T: Clone> Default for Signal<T> {
    fn default() -> Self {
        Self {
            async_fns: Default::default(),
            fns: Default::default(),
        }
    }
}

impl<T: Clone> Signal<T> {
    pub async fn emit(&self, msg: T) {
        for async_fn in &self.async_fns {
            async_fn(msg.clone()).await;
        }
        self.fns.iter().for_each(|x| x(msg.clone()));
    }

    pub fn connect_to(&mut self, receiver: impl Fn(T) + Send + Sync + 'static) {
        self.fns.push(Box::new(receiver));
    }

    pub fn connect_to_async<F>(&mut self, receiver: impl Fn(T) -> F + Send + Sync + 'static)
    where
        F: Future<Output = ()> + Send + Unpin + 'static,
    {
        self.async_fns
            .push(Box::new(move |x| Box::new(receiver(x))));
    }

    pub fn connect_to_non_blocking(&mut self, receiver: impl Fn(T) + Send + Sync + 'static)
    where
        T: Send + 'static,
    {
        let receiver = Arc::new(receiver);
        self.fns.push(Box::new(move |x| {
            let receiver = receiver.clone();
            rayon::spawn(move || receiver(x))
        }));
    }

    pub fn connect_to_async_non_blocking<F>(
        &mut self,
        receiver: impl Fn(T) -> F + Send + Sync + 'static,
    ) where
        F: Future<Output = ()> + Send + Unpin + 'static,
        T: Send + 'static,
    {
        let receiver = Arc::new(receiver);
        self.fns.push(Box::new(move |x| {
            let receiver = receiver.clone();
            tokio::spawn(async move {
                receiver(x).await;
            });
        }));
    }
}

assert_impl_all!(Signal<()>: Send, Sync);

#[derive(Deserialize, Default)]
pub struct RunOptions {
    pub log_file: Option<String>,
}

#[tokio::main]
pub async fn run_all(
    runnables: impl IntoIterator<Item = impl Node>,
    run_options: RunOptions,
) -> anyhow::Result<()> {
    let start_time = Instant::now();
    fern::Dispatch::new()
        // Add blanket level filter -
        .level(log::LevelFilter::Debug)
        // Output to stdout, files, and other Dispatch configurations
        .chain(
            fern::Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        humantime::format_rfc3339(std::time::SystemTime::now()),
                        record.level(),
                        record.target(),
                        message
                    ))
                })
                .chain(
                    run_options
                        .log_file
                        .map(|x| fern::log_file(x))
                        .unwrap_or(fern::log_file("logs.txt"))?,
                ),
        )
        .chain(
            fern::Dispatch::new()
                .format(move |out, message, record| {
                    out.finish(format_args!(
                        "[{:.2} {} {}] {}",
                        start_time.elapsed().as_secs_f32(),
                        record.level(),
                        record.target(),
                        message
                    ))
                })
                .level(log::LevelFilter::Info)
                .chain(std::io::stdout()),
        )
        // Apply globally
        .apply()?;

    let mut tasks = JoinSet::new();
    for runnable in runnables {
        tasks.spawn(async move {
            log::info!("Initializing {}", runnable.get_name());
            let name = runnable.get_name().to_owned();
            runnable.run().await.map_err(|e| (e, name))
        });
    }

    let mut ctrl_c_failed = false;

    loop {
        let ctrl_c_fut: Pin<Box<dyn Future<Output = _>>> = if ctrl_c_failed {
            Box::pin(std::future::pending())
        } else {
            Box::pin(tokio::signal::ctrl_c())
        };
        tokio::select! {
            option = tasks.join_next() => {
                let Some(result) = option else {
                    info!("All Nodes terminated. Exiting...");
                    break;
                };
                if let Err((e, name)) = result? {
                    error!("{name} has faced the following error: {e}");
                }
            }
            result = ctrl_c_fut => {
                if let Err(e) = result {
                    error!("Ctrl C handler has failed: {e}");
                    ctrl_c_failed = true;
                } else {
                    info!("Ctrl-C received. Exiting...");
                    break;
                }
            }
        }
    }

    tasks.abort_all();
    while let Some(_) = tasks.join_next().await {}
    Ok(())
}
