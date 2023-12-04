#!/bin/bash

SESSION_NAME="my-tsffs-campaign"

# Create a new tmux session or attach to an existing one
tmux new-session -d -s "$SESSION_NAME"

# Loop to create 8 windows and run the command in each window
for i in {0..7}; do
    # Create a new window
    tmux new-window -t "$SESSION_NAME:$i" -n "${SESSION_NAME}-window-$i"

    # Run the command in the new window
    tmux send-keys -t "$SESSION_NAME:$i" "./simics -no-gui --no-win --batch-mode run.simics" C-m
done

# Attach to the tmux session
tmux attach-session -t "$SESSION_NAME"
