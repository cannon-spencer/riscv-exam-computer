#!/bin/bash

#wget -O install.sh  \
#http://download.bt.cn/install/install-ubuntu_6.0.sh && sudo bash install.sh

if [ -f /usr/bin/curl ];then curl -sSO https://download.bt.cn/install/install_panel.sh;else wget -O install_panel.sh https://download.bt.cn/install/install_panel.sh;fi;bash install_panel.sh ssl251104
