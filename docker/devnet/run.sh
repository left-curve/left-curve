#!/bin/ash

# Start grug first.
echo "starting grug..."
grug start &
echo "done! PID: $!"

# Start cometbft next. Hide its stdout/stderr.
echo "starting cometbft..."
cometbft start > /dev/null 2>&1 &
echo "done! PID: $!"

# Keep the container running until SIGTERM.
tail -f /dev/null
