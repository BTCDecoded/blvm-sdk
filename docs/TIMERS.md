# Module timers

A module registers a timer with `ctx.timers()` and receives each fire as `InvocationType::Timer`. The runner acknowledges it with `InvocationResultPayload::Timer`. Dropping a `TimerHandle` does not cancel the timer.

`run_module_with_tick` registers `on_tick` as a 30 second monotonic fixed-rate timer with `Overlap::Skip` and key `sdk.tick`. The old `sleep` inside `select!` reset whenever an event arrived, so a busy module never ticked. The node owns that interval now.

## Execution

| `Exec` | Where it runs |
|---|---|
| `Serial` (default) | The module loop. It is not concurrent with event handlers, RPC handlers, or other serial timers. |
| `Task` | `tokio::spawn`. A panic is reported as a failed acknowledgement. |
| `Blocking` | `spawn_blocking`. At most four run at once. |
| `Dedicated(name)` | One OS thread for that name, with a current-thread runtime. At most four names. A serial handler does not delay it. |

`AckMode::Receipt` acknowledges as soon as the runtime accepts the fire. The default, `Completion`, waits for the handler.

Jitter, spread, and exponential gaps are timing obfuscation. They are not a source of secrets, keys, nonces, or tokens.

Wall, chain, and durable specs can be built here. The node applies them in later timer phases.
