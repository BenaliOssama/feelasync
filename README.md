# feelasync

A tiny async runtime built from first principles — going all the way down to the bones.

The goal is not to ship a production runtime. The goal is to *feel* how async works, layer by layer, starting from the hardware and climbing up until a real executor is sitting in front of you.

---

## The idea

Modern async runtimes feel like magic until you trace them back to what is actually happening. A `Future` suspended on `.await` is ultimately just a task the OS woke up when a hardware event fired — an interrupt, handled by the kernel, surfaced through a syscall, caught by a reactor, which nudges a waker, which re-queues the task for the executor.

This project is that chain, written by hand, in Rust, with no async/await and no Tokio.

---

## The layers (bottom up)

```
hardware interrupt
      ↓
  kernel (IRQ handler, device driver)
      ↓
  epoll  (Linux's I/O readiness notification — epoll_create / epoll_wait)
      ↓
  Reactor  (owns the epoll fd, maps fd → Waker)
      ↓
  Waker    (a tiny handle that pushes a TaskId onto the ready queue)
      ↓
  Executor (poll loop: drain ready queue → call task.poll() → sleep in reactor)
      ↓
  Task     (your async logic, as a hand-rolled state machine)
```

The file layout mirrors these layers directly:

| file | responsibility |
|---|---|
| `reactor.rs` | owns epoll, maps `fd → Waker`, calls `waker.wake()` on I/O events |
| `waker.rs` | a clonable handle that pushes a `TaskId` onto the executor's ready queue |
| `executor.rs` | poll loop + `Spawner` so tasks can spawn other tasks |
| `timer.rs` | a one-shot timer task built on `timerfd_create` |
| `main.rs` | a non-blocking TCP echo server — the demo that exercises all of it |

---

## What works right now

- `epoll`-based reactor wrapping raw `libc` calls
- Single-threaded executor with a ready queue
- Tasks spawning other tasks at runtime (via `Spawner`)
- `timerfd`-backed timer tasks
- A TCP echo server: accepts connections, spawns an `EchoTask` per socket, all running concurrently inside the one loop

```
$ cargo run
listening on 127.0.0.1:7878
>>> timer [three-sec] fired        # after 3 s
new conn fd 7 from 127.0.0.1:XXXXX
fd 7 read 13 bytes
>>> timer [seven-sec] fired        # after 7 s
```

---

## What is missing / still ahead

This is a work in progress. Layers still to climb:

- [ ] Proper `Future` / `Poll` traits (align with Rust's std model)
- [ ] Combinators (`join`, `select`, timeout wrappers)
- [ ] Async write support
- [ ] Actual `async fn` integration via the `Future` trait
- [ ] Understanding where `Pin` fits and why

---

## Why "feelasync"

Because the point is to *feel* it — not read about it, not import it, but wire it yourself from the syscall up and watch it move.

---

## Stack

- Rust (no async/await, no Tokio, no futures crate)
- `libc` for raw syscalls (`epoll_*`, `timerfd_*`)
- Linux only (epoll is Linux-specific)
