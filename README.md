# feelasync

A tiny async runtime in Rust, built from scratch to understand how runtimes
actually work.

This is a learning project. The goal is not to compete with tokio — the goal
is to demystify it. By building each piece bottom-up from raw `epoll` syscalls
up to working `async fn`, the magic of `async`/`await` and `Future` becomes
just structure you've written yourself.

## What this does

- Single-threaded async runtime, ~400 lines of Rust.
- Handles many concurrent TCP connections on one thread.
- Supports timers via `timerfd`.
- 0% CPU when idle. Parks on `epoll_wait` and is woken by hardware interrupts.
- Accepts real Rust `async fn` and `.await`.
- No dependencies except `libc`.

## Architecture

Three pieces talking through a fourth:

- **Reactor** (`src/reactor.rs`) — owns the epoll instance. Maps `fd → Waker`.
  When an fd becomes ready, calls its waker. Knows nothing about tasks.
- **Executor** (`src/executor.rs`) — owns tasks and the ready queue. Polls
  tasks. When nothing is ready, asks the Reactor to park. Knows nothing
  about epoll.
- **Waker** (`src/waker.rs`) — the wire between them. A small handle that
  pushes a task id onto the ready queue when called.
- **Tasks** — anything implementing `std::future::Future`. The compiler
  generates these for you when you write `async fn`.

The whole event flow:

```
packet arrives at NIC
  → hardware interrupt → kernel handler → fd marked ready
  → executor's epoll_wait returns
  → reactor looks up fd's waker → waker.wake() → task id on ready queue
  → executor pops the id → polls the task
  → task does work → returns Ready or Pending
```

## Run it

```bash
cargo run
```

Then in another terminal:

```bash
nc 127.0.0.1 7878
```

Type bytes, see them logged. Two timers fire after 3s and 7s. CPU stays at
0% between events.

## File structure

```
src/
├── main.rs       — entry point; async functions for the echo server
├── executor.rs   — task storage, ready queue, run loop, Spawner
├── reactor.rs    — epoll wrapper, fd → waker map
├── waker.rs      — std::task::Waker construction via RawWakerVTable
├── io.rs         — Readable future (await on fd readiness)
└── timer.rs      — TimerTask, an fd-based timer future
```

## Why bottom-up

I tried understanding async top-down through tutorials. It didn't stick.
Every abstraction felt like ceremony. So I went the other way: started with
a single blocking socket read, watched the kernel park my thread, kept
adding pressure until each abstraction was a real fix to a real problem.

Each commit in this repo corresponds to one step in that journey. The
git log tells the story.

## Blog series

- [Part 1: Feeling async at the kernel level](./blog/part1.md)
  — five tiny programs, no abstractions. Blocking vs non-blocking vs epoll.
  How `timerfd` unifies time and I/O.

- [Part 2: Building the runtime](./blog/part2.md)
  — Reactor, Executor, Waker, Tasks. From manual dispatcher to working
  `async fn`.

## Where this could go next

- Replace epoll with `io_uring`.
- Multi-threaded executor with work-stealing.
- Add a SignalTask using `signalfd`.
- Channel-based wakeups between tasks (`Sender`/`Receiver`).
- Compare with tokio's `Runtime`, `Scheduler`, `Driver` source.

## Not for production

This is a learning artifact. For real work use `tokio`, `smol`, or
`async-std`. Read this to understand them.

## Built by

[Osben](https://github.com/BenaliOssama) — a backend/systems engineer
working through Rust async from first principles.
