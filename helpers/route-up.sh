#!/bin/sh

sudo ip route add 188.40.167.82 dev tun0
sudo iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
sudo iptables -A FORWARD -i tun0 -j ACCEPT
