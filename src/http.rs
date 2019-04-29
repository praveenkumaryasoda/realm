use antidote::RwLock;
use futures_cpupool::{self, CpuPool};
use lazy_static::lazy_static;
use thread_id;

lazy_static! {
    pub static ref THREAD_POOL: CpuPool = {
        let mut builder = futures_cpupool::Builder::new();
        builder.pool_size(2);
        builder.stack_size(16 * 1024 * 1024); // 16mb, default is 8mb
        builder.create()
    };

    pub static ref GLOBALS: RwLock<i32> = {
        RwLock::new(42)
    };
}

pub fn tid() -> usize {
    thread_id::get()
}

#[macro_export]
macro_rules! realm {
    ($e:expr) => {
        pub fn main() {
            use futures::{self, IntoFuture};
            use hyper::{self, rt::Future, Body};
            use realm::{self, GLOBALS, THREAD_POOL};
            use std::{self, thread};

            type BoxFut = Box<Future<Item = hyper::Response<Body>, Error = hyper::Error> + Send>;

            pub fn handle_sync(
                _req: hyper::Request<Body>,
            ) -> std::result::Result<hyper::Response<Body>, hyper::Error> {
                let req: realm::Request = unimplemented!();
                Ok($e(req)
                    .map(|r| r.to_hyper())
                    .unwrap_or_else(|e| e.to_hyper()))
            }

            pub fn serve() {
                let addr = ([127, 0, 0, 1], 3000).into();
                println!("main_: {:?}, tid: {}", thread::current().id(), realm::tid());

                let server = hyper::Server::bind(&addr)
                    .serve(|| {
                        hyper::service::service_fn(|req: hyper::Request<Body>| -> BoxFut {
                            println!("future tid: {:?}, {}", thread::current().id(), realm::tid());
                            Box::new(THREAD_POOL.spawn_fn(|| {
                                let mut i = GLOBALS.write();
                                *i += 1;
                                let tid = thread::current().id();
                                println!("threadid: {:?} {}", thread::current().id(), realm::tid());
                                println!("yo: {}, tid: {:?}", *i, tid);
                                handle_sync(req).into_future()
                            }))
                        })
                    }).map_err(|e| eprintln!("server error: {}", e));

                println!("Listening on http://{}", addr);
                hyper::rt::run(server);
            }

            serve()
        }
    };
}
