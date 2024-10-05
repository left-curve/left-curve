#!/bin/ash

# When starting, start grug first, then comet.
# When stopping, stop comet first, then grug.

# Gracefully stop the processes when the container is shut down with CTRL+C.
_term() {
  echo "caught SIGTERM signal!"

  echo "stopping cometbft..."
  kill $COMET_PID
  echo "done"

  echo "stopping grug..."
  kill $GRUG_PID
  echo "done"

  exit 0
}

# Start grug first. Redirect the stdout/stderr to the container's stdout/stderr.
echo "starting grug..."
grug start > >(tee -a /proc/self/fd/1) 2> >(tee -a /proc/self/fd/2 >&2) &
GRUG_PID=$!

# Start cometbft next. Hide its stdout/stderr.
echo "starting cometbft..."
cometbft start > /dev/null 2>&1 &
COMET_PID=$!

# Keep the container running until SIGTERM.
tail -f /dev/null
