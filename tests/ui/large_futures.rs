#![allow(clippy::manual_async_fn)]
#![warn(clippy::large_futures)]

// Note: large_futures are allowed here, as rustfix cannot actually fix this case.
// The reason we still keep it around is that it's used as a helper in other tests.
// See large_futures_unfixable.rs where this definition is demonstrated to emit a lint as well.
#[allow(clippy::large_futures)]
async fn big_fut(_arg: [u8; 1024 * 16]) {}

async fn wait() {
    let f = async {
        big_fut([0u8; 1024 * 16]).await;
        //~^ large_futures
    };
    f.await
    //~^ large_futures
}
async fn calls_fut(fut: impl std::future::Future<Output = ()>) {
    loop {
        wait().await;
        //~^ large_futures

        if true {
            return fut.await;
        } else {
            wait().await;
            //~^ large_futures
        }
    }
}

pub async fn test() {
    let fut = big_fut([0u8; 1024 * 16]);
    //~^ large_futures

    foo().await;
    //~^ large_futures

    calls_fut(fut).await;
    //~^ large_futures
}

pub fn foo() -> impl std::future::Future<Output = ()> {
    //~v large_futures
    async {
        let x = [0i32; 1024 * 16];
        async {}.await;
        dbg!(x);
    }
}

pub async fn lines() {
    //~v large_futures
    async {
        let x = [0i32; 1024 * 16];
        async {}.await;

        println!("{:?}", x);
    }
    .await;
}

// Note: large_futures are allowed here, as rustfix cannot actually fix this case.
pub async fn macro_expn() {
    macro_rules! macro_ {
        () => {
            //~v large_futures
            async {
                let x = [0i32; 1024 * 16];
                async {}.await;
                println!("macro: {:?}", x);
            }
        };
    }
    macro_!().await
}

fn main() {}
