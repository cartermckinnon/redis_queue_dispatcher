# redis queue dispatcher

![Rust](https://github.com/cartermckinnon/redis_queue_dispatcher/workflows/Rust/badge.svg)

A "dispatcher" for a delayed task queue built on Redis, written in Rust.

There are 3 functions involved in the delayed/scheduled task queue [described in the Redis e-book](https://redislabs.com/ebook/part-2-core-concepts/chapter-6-application-components-in-redis/6-4-task-queues/6-4-2-delayed-tasks/):
- `a`: Add tasks for future execution to a sorted set.
- `b`: Poll the sorted set for tasks which are ready for execution, moving them to a list.
- `c`: Remove tasks from the list and execute them.

This project implements `b`, as the implementations of `a` and `c` depend on your use-case. The following are example implementations:
- `a`: `ZADD delayedtaskset 1 taskA`
- `c`: `BLPOP readytasklist`

### Build

```sh
cargo build --release
```

### Usage

```
redis_queue_dispatcher [OPTIONS]

Dispatch tasks from a delayed task queue.

Optional arguments:
  -h,--help             Show this help message and exit
  -v,--verbose          Verbose output
  -u,--redis-url REDIS_URL
                        Redis URL (default: 'redis://localhost:6379').
  -d,--delayed-task-zset-key DELAYED_TASK_ZSET_KEY
                        Key of delayed task ZSET (default: 'delayedtaskset').
  -r,--ready-task-list-key READY_TASK_LIST_KEY
                        Key of ready task LIST (default: 'readytasklist').
  -n,--batch-size BATCH_SIZE
                        Number of tasks to dispatch at once (default: 10).
```

### Notes

Lua scripting is used to implement an atomic "dispatch" operation. Lua is used because it is (a) faster than locks, and (b) is less complicated (in the opinion of the author). This operation can be performed on more than one element at a time, to reduce round-trips.

Polling overhead should be kept to a minimum, meaning only a small number of this program should run simultaneously. If there are no tasks to be dispatched, polling is less frequent (exponential backoff).
