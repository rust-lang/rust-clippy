#![warn(clippy::disallowed_from_async)]
extern crate tokio;

use tokio::runtime::Handle;

pub async fn insulated_block_on_example_direct() {
    let handle1 = Handle::current();
    let handle2 = handle1.clone();

    handle1
        .spawn_blocking(move || handle2.block_on(async move { println!("hello from block_on") }))
        .await
        .unwrap()
}

pub async fn insulated_block_on_example_indirect_closures() {
    let handle1 = Handle::current();
    let handle2 = handle1.clone();

    let task = async move { println!("hello from block_on") };
    let f = move || handle2.block_on(task);

    handle1.spawn_blocking(f).await.unwrap()
}

pub async fn uninsulated_nested_block_on_direct() {
    let handle1 = Handle::current();
    let handle2 = handle1.clone();
    let handle3 = handle1.clone();

    handle1
        .spawn_blocking(move || {
            handle2.block_on(async move { handle3.block_on(async move { println!("naughty block_on") }) })
        })
        .await
        .unwrap()
}

pub async fn uninsulated_nested_block_on_indirect_closures() {
    let handle1 = Handle::current();
    let handle2 = handle1.clone();
    let handle3 = handle1.clone();

    let task2 = async move { println!("naughty block_on") };
    let task1 = async move { handle3.block_on(task2) };
    let f = move || handle2.block_on(task1);

    handle1.spawn_blocking(f).await.unwrap()
}

pub async fn uninsulated_nested_block_on_indirect_functions() {
    async fn task2() {
        println!("hello");
    }

    async fn task1() {
        let handle = Handle::current();
        handle.block_on(task2());
    }

    fn f() {
        let handle = Handle::current();
        handle.block_on(task1());
    }

    Handle::current().spawn_blocking(f).await.unwrap()
}

fn main() {}
