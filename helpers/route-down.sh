#!/bin/sh

sudo ip route del 188.40.167.82 dev eth0

sudo iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE
sudo iptables -D FORWARD -i tun0 -j ACCEPT
