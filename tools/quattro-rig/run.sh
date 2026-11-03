#!/bin/bash
# usage: run.sh NAME SECONDS 'DOS commands separated by ;;'
# Starts DOSBox-X on $QP_RIG/c, waits, and screenshots its window to $QP_RIG/shots/NAME.png.
# Leaves DOSBox-X running if KEEP is set.
set -u
: "${QP_RIG:?set QP_RIG to the rig folder}"
n=$1; secs=$2; cmds=$3
pkill -x dosbox-x; sleep 0.5
{
  cat <<C
[sdl]
autolock=false
priority=higher,normal
[dosbox]
memsize=16
machine=svga_s3
quit warning=false
[cpu]
cycles=max
[dos]
files=60
[autoexec]
mount c "$QP_RIG/c" -nocachedir
c:
C
  printf '%s' "$cmds" | sed 's/;;/\n/g'; echo
} > "$QP_RIG/$n.conf"
(cd "$QP_RIG" && dosbox-x -conf "$n.conf" -nopromptfolder -fastlaunch > dbx.log 2>&1 &)
sleep "$secs"
id=$("$QP_RIG/wid"); screencapture -x -l"$id" "$QP_RIG/shots/$n.png"
[ -n "${KEEP:-}" ] || pkill -x dosbox-x
