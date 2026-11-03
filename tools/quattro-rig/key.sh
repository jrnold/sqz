#!/bin/bash
# usage: key.sh NAME 'keys' 'DOS commands (;; separated)' SECONDS...
# Types `keys` (see mk.py) into a DOS session, and takes a screenshot at each SECONDS mark.
set -u
: "${QP_RIG:?set QP_RIG to the rig folder}"
here=$(cd "$(dirname "$0")" && pwd)
n=$1; k=$2; cmds=$3; shift 3
python3 "$here/mk.py" "$QP_RIG/c/KEYS.DAT" "$k"
KEEP=1 "$here/run.sh" "$n" 2 "KEYS.COM;;$cmds" >/dev/null
id=$("$QP_RIG/wid"); e=2
for t in "$@"; do
  sleep $((t - e)); e=$t
  screencapture -x -l"$id" "$QP_RIG/shots/$n-$t.png"
done
pkill -x dosbox-x
