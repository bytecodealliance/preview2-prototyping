use anyhow::Result;
use cap_rand::RngCore;
use cap_std::{fs::Dir, time::Duration};
use host::{add_to_linker, Wasi, WasiCtx};
use std::{
    io::{Cursor, Write},
    sync::Mutex,
};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{
    clocks::{WasiMonotonicClock, WasiWallClock},
    pipe::ReadPipe,
};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

test_programs_macros::tests!();

async fn instantiate(path: &str) -> Result<(Store<WasiCtx>, Wasi)> {
    println!("{}", path);

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &path)?;
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker, |x| x)?;

    let mut store = Store::new(&engine, WasiCtxBuilder::new().build());

    let (wasi, _instance) = Wasi::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, wasi))
}

async fn run_hello_stdout(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    wasi.command(
        &mut store,
        0 as host::WasiStream,
        1 as host::WasiStream,
        &["gussie", "sparky", "willa"],
        &[],
        &[],
    )
    .await?;
    Ok(())
}

async fn run_panic(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    let r = wasi
        .command(
            &mut store,
            0 as host::WasiStream,
            1 as host::WasiStream,
            &[
                "diesel",
                "the",
                "cat",
                "scratched",
                "me",
                "real",
                "good",
                "yesterday",
            ],
            &[],
            &[],
        )
        .await;
    assert!(r.is_err());
    println!("{:?}", r);
    Ok(())
}

async fn run_args(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    wasi.command(
        &mut store,
        0 as host::WasiStream,
        1 as host::WasiStream,
        &["hello", "this", "", "is an argument", "with 🚩 emoji"],
        &[],
        &[],
    )
    .await?;
    Ok(())
}

async fn run_random(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    struct FakeRng;

    impl RngCore for FakeRng {
        fn next_u32(&mut self) -> u32 {
            42
        }

        fn next_u64(&mut self) -> u64 {
            unimplemented!()
        }

        fn fill_bytes(&mut self, _dest: &mut [u8]) {
            unimplemented!()
        }

        fn try_fill_bytes(&mut self, _dest: &mut [u8]) -> Result<(), cap_rand::Error> {
            unimplemented!()
        }
    }

    store.data_mut().random = Box::new(FakeRng);

    wasi.command(
        &mut store,
        0 as host::WasiStream,
        1 as host::WasiStream,
        &[],
        &[],
        &[],
    )
    .await?;
    Ok(())
}

async fn run_time(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    struct FakeWallClock;

    impl WasiWallClock for FakeWallClock {
        fn resolution(&self) -> Duration {
            Duration::from_secs(1)
        }

        fn now(&self) -> Duration {
            Duration::from_secs(1431648000)
        }

        fn dup(&self) -> Box<dyn WasiWallClock + Send + Sync> {
            Box::new(Self)
        }
    }

    struct FakeMonotonicClock {
        now: Mutex<u64>,
    }

    impl WasiMonotonicClock for FakeMonotonicClock {
        fn resolution(&self) -> u64 {
            1_000_000_000
        }

        fn now(&self) -> u64 {
            let mut now = self.now.lock().unwrap();
            let then = *now;
            *now += 42 * 1_000_000_000;
            then
        }

        fn dup(&self) -> Box<dyn WasiMonotonicClock + Send + Sync> {
            let now = *self.now.lock().unwrap();
            Box::new(Self {
                now: Mutex::new(now),
            })
        }
    }

    store.data_mut().clocks.default_wall_clock = Box::new(FakeWallClock);
    store.data_mut().clocks.default_monotonic_clock =
        Box::new(FakeMonotonicClock { now: Mutex::new(0) });

    wasi.command(
        &mut store,
        0 as host::WasiStream,
        1 as host::WasiStream,
        &[],
        &[],
        &[],
    )
    .await?;
    Ok(())
}

async fn run_stdin(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    store
        .data_mut()
        .set_stdin(Box::new(ReadPipe::new(Cursor::new(
            "So rested he by the Tumtum tree",
        ))));

    wasi.command(
        &mut store,
        0 as host::WasiStream,
        1 as host::WasiStream,
        &[],
        &[],
        &[],
    )
    .await?;
    Ok(())
}

async fn run_env(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &[],
        &[("frabjous", "day"), ("callooh", "callay")],
        &[],
    )
    .await?;
    Ok(())
}

async fn run_file_read(mut store: Store<WasiCtx>, wasi: Wasi) -> Result<()> {
    let dir = tempfile::tempdir()?;

    std::fs::File::create(dir.path().join("bar.txt"))?.write_all(b"And stood awhile in thought")?;

    let descriptor =
        store
            .data_mut()
            .push_dir(Box::new(wasi_cap_std_sync::dir::Dir::from_cap_std(
                Dir::from_std_file(std::fs::File::open(dir.path())?),
            )))?;

    wasi.command(
        &mut store,
        0 as host::Descriptor,
        1 as host::Descriptor,
        &[],
        &[],
        &[(descriptor, "/")],
    )
    .await?;
    Ok(())
}
