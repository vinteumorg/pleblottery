# tokio-console

`pleblottery` aims to be robust async system programming

we leverage [`tokio-console`](https://github.com/tokio-rs/console) instrumentation to gather insights about code architecture.

`scripts/tokio_console.sh` does the following:
- launch [`tmux`](https://github.com/tmux/tmux/wiki) in a split screen
- launch `pleblottery` with `tokio_debug` feature flag enabled on first pane
- launch `tokio-console` for monitoring tokio tasks on second pane

![](./img/tokio-console.png)