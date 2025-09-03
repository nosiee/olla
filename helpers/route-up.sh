#!/bin/sh

sudo ip route add 0.0.0.0/1 dev tun0
sudo ip route add 45.84.88.207 dev eth0

sudo iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
sudo iptables -A FORWARD -i tun0 -j ACCEPT
