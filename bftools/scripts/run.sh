#! /bin/bash

trap quit SIGINT SIGTERM SIGQUIT

if test -z "$1"; then
    echo "usage: run.sh <name>"
    exit 1
fi

if echo "$1" | grep -Eq "[:space:]"; then
    echo "name may not contain white space"
    exit 2
fi

NAME="$1"
DCS="~/.wine/drive_c/Program Files/Eagle Dynamics/DCS World OpenBeta Server"
DATA="~/.wine/drive_c/users/dcs/Saved Games/$NAME"

function kill_dcs() {
    PID=$(ps auxwww \
              | grep "DCS_server.exe -w $NAME" \
              | grep -v grep \
              | awk '{pring $2}')
    kill $PID
}

function quit() {
    kill_dcs
    jobs | xargs kill
}

function watchdog() {
    while true; do
        TS=$(stat -c "%Y" "${DATA}/Logs/bfnext.txt")
        NOW=$(date +%s)
        if test $((NOW - TS)) -gt 300; then
            kill_dcs
        fi
        sleep 60
    done 
}

function run_dcs() {
    cd "$DCS"
    while true; do
        wine bin/DCS_server.exe -w "$NAME"
        sleep 10
    done
}

watchdog &
run_dcs &
wait
