#!/bin/bash

find . -name 'server.cfg' -exec grep -H SERVER_PORT {} \; |\
    sort -t= -k2 -g | \
    while IFS='\n' read R; do
        D=$(dirname "$R")
        grep -H "SRSAuto.SERVER_SRS_PORT *= *" "${D}/DCS-SRS-AutoConnectGameGUI.lua" | sed -e 's/--.*$//'
        echo "$R"
        echo ""
    done
