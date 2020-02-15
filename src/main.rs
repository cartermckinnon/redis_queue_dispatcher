extern crate argparse;
extern crate ctrlc;
extern crate redis;

use argparse::{ArgumentParser, Store, StoreTrue};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let mut verbose = false;
    let mut redis = "redis://localhost:6379".to_string();
    let mut delayed = "delayedtaskset".to_string();
    let mut ready = "readytasklist".to_string();
    let mut n = 10;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Dispatch tasks from a delayed task queue.");
        ap.refer(&mut verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "Verbose output");
        ap.refer(&mut redis).add_option(
            &["-u", "--redis-url"],
            Store,
            "Redis URL (default: 'redis://localhost:6379').",
        );
        ap.refer(&mut delayed).add_option(
            &["-d", "--delayed-task-zset-key"],
            Store,
            "Key of delayed task ZSET (default: 'delayedtaskset').",
        );
        ap.refer(&mut ready).add_option(
            &["-r", "--ready-task-list-key"],
            Store,
            "Key of ready task LIST (default: 'readytasklist').",
        );
        ap.refer(&mut n).add_option(
            &["-n", "--batch-size"],
            Store,
            "Number of tasks to dispatch at once (default: 10).",
        );
        ap.parse_args_or_exit();
    }
    let client = redis::Client::open(redis.clone()).unwrap();
    let mut conn = client.get_connection().unwrap();
    println!("Connected to Redis at '{}'", redis);
    let script = redis::Script::new(SCRIPT);

    let one_second = std::time::Duration::from_secs(1);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting CTRL-C handler");

    println!(
        "Dispatching up to {} tasks per batch from ZSET '{}' to LIST '{}'...",
        &n, &delayed, &ready
    );

    let mut now;
    let backoff = [0, 1, 1, 2, 3, 5, 8, 13];
    let mut b = 0; // current position in backoff[]
    let mut seconds_backed_off = 0;
    let lines_without_header = 50;
    let mut lines_since_header = 50; // trigger initial headers

    while running.load(Ordering::SeqCst) {
        now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let result: redis::RedisResult<(u64, u64, u64)> = script
            .prepare_invoke()
            .key(&delayed)
            .key(&ready)
            .arg(now)
            .arg(n)
            .invoke(&mut conn);
        match result {
            Ok(res) => {
                // res.0 == dispatched
                // res.1 == delayed
                // res.2 == ready
                if res.0 > 0 {
                    b = 0; // reset backoff
                } else if b < backoff.len() - 1 {
                    b += 1; // backoff
                }
                if verbose {
                    if lines_since_header == lines_without_header {
                        println!(
                            "+---------------+---------------+---------------+---------------+"
                        );
                        println!(
                            "{}",
                            format!(
                                "|{:^15}|{:^15}|{:^15}|{:^15}|",
                                "dispatched", "delayed", "ready", "backing_off"
                            )
                        );
                        println!(
                            "+---------------+---------------+---------------+---------------+"
                        );
                        lines_since_header = 0;
                    } else {
                        lines_since_header += 1;
                    }
                    println!(
                        "{}",
                        format!(
                            "|{:^15}|{:^15}|{:^15}|{:^15}|",
                            res.0, res.1, res.2, backoff[b]
                        )
                    );
                }
            }
            Err(e) => {
                println!("Failed to dispatch tasks: {}", e);
                break;
            }
        }

        // allow the program to terminate quickly after CTRL-C (within 1 second)
        while seconds_backed_off < backoff[b] && running.load(Ordering::SeqCst) {
            std::thread::sleep(one_second);
            seconds_backed_off += 1;
        }
        seconds_backed_off = 0;
    }
    if verbose && lines_since_header > 0 {
        println!("+---------------+---------------+---------------+---------------+");
    }
    println!("Exiting!");
}

const SCRIPT: &str = r"
local ready = redis.call('zrangebyscore',KEYS[1], 0, tonumber(ARGV[1]),'limit',0,tonumber(ARGV[2]))
local dispatched = 0
if ready ~= nil and #ready > 0 then
  for score,host in pairs(ready) do
    redis.call('rpush', KEYS[2], host)
    redis.call('zrem', KEYS[1], host)
  end
  dispatched = #ready
end
local delayed = redis.call('zcard', KEYS[1])
local ready_prev = redis.call('llen', KEYS[2]) - dispatched
return {dispatched, delayed, ready_prev}";
