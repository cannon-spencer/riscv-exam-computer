#!/bin/bash

[[ ! -c "$1" ]] && exit

DISPLAY=:0 gst-launch-1.0 v4l2src device=$1 en-awisp=1 en-largemode=0 ! video/x-raw,format=NV12,width=640,height=480,framerate=30/1 ! xvimagesink

