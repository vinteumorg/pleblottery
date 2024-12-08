#!/bin/sh

# Script creates a tmux session with horizontal split
# Left pane runs pleblottery with tokio_debug flag
# Right pane runs tokio_console to monitor pleblottery tokio tasks

# Check if tmux is installed
if ! command -v tmux &> /dev/null; then
    echo "tmux is not installed. Please install it and try again."
    exit 1
fi

TMUX_SESSION="pleblottery_tokio_debug"

CMD_PLEBLOTTERY="RUSTFLAGS=\"--cfg tokio_unstable\" cargo watch -c -x 'r --features tokio_debug'"
CMD_TOKIO_CONSOLE="tokio-console"

# Start a new tmux session detached
tmux new-session -d -s "$TMUX_SESSION"

# Run pleblottery in the first (default) pane
tmux send-keys -t "$TMUX_SESSION:0.0" "$CMD_PLEBLOTTERY" C-m

# Split the window horizontally
tmux split-window -h -t "$TMUX_SESSION:0"

# Run tokio-console in the second pane
tmux send-keys -t "$TMUX_SESSION:0.1" "$CMD_TOKIO_CONSOLE" C-m

# Attach to the tmux session
tmux attach -t "$TMUX_SESSION"