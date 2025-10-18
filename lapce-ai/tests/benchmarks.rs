// #![feature(test)] // Only on nightly
extern crate test;

#[bench]
fn bench_message_send(b: &mut test::Bencher) {
    let msg = vec![0u8; 1024];
    b.iter(|| {
        send_message(&msg);
    });
}

#[bench]
fn bench_cache_lookup(b: &mut test::Bencher) {
    b.iter(|| {
        cache_lookup("test_key");
    });
}

fn send_message(_msg: &[u8]) {}
fn cache_lookup(_key: &str) -> Option<String> { None }
