#!/bin/sh

sudo ip route del 0.0.0.0/1 dev tun0
sudo ip route del 45.84.88.207 dev eth0

sudo iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE
sudo iptables -D FORWARD -i tun0 -j ACCEPT
