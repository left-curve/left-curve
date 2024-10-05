#!/bin/ash

# Start grug first. Redirect the stdout/stderr to the container's stdout/stderr.
echo "starting grug..."
grug start > >(tee -a /proc/self/fd/1) 2> >(tee -a /proc/self/fd/2 >&2) &
echo "done! PID: $!"

# Start cometbft next. Hide its stdout/stderr.
echo "starting cometbft..."
cometbft start > /dev/null 2>&1 &
echo "done! PID: $!"

# Keep the container running until SIGTERM.
tail -f /dev/null
