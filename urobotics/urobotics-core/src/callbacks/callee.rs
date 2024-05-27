use std::{ops::ControlFlow, sync::Arc};

use crossbeam::queue::ArrayQueue;
use tokio::sync::Notify;

struct SubscriberInner<T> {
    queue: ArrayQueue<T>,
    notify: Notify,
}

pub struct Subscriber<T> {
    inner: Arc<SubscriberInner<T>>,
}

impl<T> Subscriber<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Arc::new(SubscriberInner {
                queue: ArrayQueue::new(max_size),
                notify: Notify::new(),
            }),
        }
    }

    #[inline]
    pub fn try_recv(&self) -> Option<T> {
        self.inner.queue.pop()
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        Arc::weak_count(&self.inner) == 0
    }

    pub async fn recv(&self) -> Option<T> {
        loop {
            if let Some(value) = self.inner.queue.pop() {
                return Some(value);
            }

            if self.is_closed() {
                return None;
            }

            self.inner.notify.notified().await;
        }
    }

    pub async fn recv_or_never(&self) -> T {
        if let Some(value) = self.recv().await {
            value
        } else {
            std::future::pending().await
        }
    }

    pub fn create_conservative_callback(&self) -> impl Fn(T) -> ControlFlow<()> + Send + Sync
    where
        T: Send,
    {
        let inner = Arc::downgrade(&self.inner.clone());
        move |value| {
            let Some(inner) = inner.upgrade() else {
                return ControlFlow::Break(());
            };
            if inner.queue.push(value).is_ok() {
                inner.notify.notify_one();
            }
            ControlFlow::Continue(())
        }
    }

    pub fn create_callback(&self) -> impl Fn(T) -> ControlFlow<()> + Send + Sync
    where
        T: Send,
    {
        let inner = Arc::downgrade(&self.inner.clone());
        move |value| {
            let Some(inner) = inner.upgrade() else {
                return ControlFlow::Break(());
            };
            if inner.queue.force_push(value).is_none() {
                inner.notify.notify_one();
            }
            ControlFlow::Continue(())
        }
    }

    #[inline]
    pub fn create_conservative_mut_callback(&self) -> impl FnMut(T) -> ControlFlow<()> + Send + Sync
    where
        T: Send,
    {
        self.create_conservative_callback()
    }

    #[inline]
    pub fn create_mut_callback(&self) -> impl FnMut(T) -> ControlFlow<()> + Send + Sync
    where
        T: Send,
    {
        self.create_callback()
    }
}