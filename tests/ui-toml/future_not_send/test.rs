//@revisions: default uncond
//@[default] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/future_not_send/default
//@[uncond] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/future_not_send/unconditional_send_futures
#![warn(clippy::future_not_send)]

use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

async fn private_future(rc: Rc<[u8]>, cell: &Cell<usize>) -> bool {
    //~[uncond]^ future_not_send
    //~[default]| future_not_send
    async { true }.await
}

pub async fn public_future(rc: Rc<[u8]>) {
    //~[uncond]^ future_not_send
    //~[default]| future_not_send
    async { true }.await;
}

pub async fn public_send(arc: Arc<[u8]>) -> bool {
    async { false }.await
}

async fn private_future2(rc: Rc<[u8]>, cell: &Cell<usize>) -> bool {
    //~[uncond]^ future_not_send
    //~[default]| future_not_send
    true
}

pub async fn public_future2(rc: Rc<[u8]>) {}
//~[uncond]^ future_not_send
//~[default]| future_not_send

pub async fn public_send2(arc: Arc<[u8]>) -> bool {
    false
}

struct Dummy {
    rc: Rc<[u8]>,
}

impl Dummy {
    async fn private_future(&self) -> usize {
        //~[uncond]^ future_not_send
        //~[default]| future_not_send
        async { true }.await;
        self.rc.len()
    }

    pub async fn public_future(&self) {
        //~[uncond]^ future_not_send
        //~[default]| future_not_send
        self.private_future().await;
    }

    #[allow(clippy::manual_async_fn)]
    pub fn public_send(&self) -> impl std::future::Future<Output = bool> {
        async { false }
    }
}

async fn generic_future<T>(t: T) -> T
//~[uncond]^ future_not_send
//~[default]| future_not_send
where
    T: Send,
{
    let rt = &t;
    async { true }.await;
    let _ = rt;
    t
}

async fn maybe_send_generic_future<T>(t: T) -> T {
    //~[uncond]^ future_not_send
    async { true }.await;
    t
}

async fn maybe_send_generic_future2<F: Fn() -> Fut, Fut: Future>(f: F) {
    //~[uncond]^ future_not_send
    async { true }.await;
    let res = f();
    async { true }.await;
}

async fn generic_future_always_unsend<T>(_: Rc<T>) {
    //~[uncond]^ future_not_send
    //~[default]| future_not_send
    async { true }.await;
}

async fn generic_future_send<T>(t: T)
where
    T: Send,
{
    async { true }.await;
}

async fn unclear_future<T>(t: T) {}
//~[uncond]^ future_not_send

fn main() {
    let rc = Rc::new([1, 2, 3]);
    private_future(rc.clone(), &Cell::new(42));
    public_future(rc.clone());
    let arc = Arc::new([4, 5, 6]);
    public_send(arc);
    generic_future(42);
    generic_future_send(42);

    let dummy = Dummy { rc };
    dummy.public_future();
    dummy.public_send();
}
