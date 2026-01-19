The main idea was to distribute traffic between multiple nodes, servers. So for your ISP everything looks like you're talking to multiple servers not a single one.
All traffic is distributed randomly between nodes, but you must set a primary node. The primary node is the node that will send initial request to the target server and send you the response.

Some pretty important TODO's, which i am unlikely to do:

1) traffic in the tunnel is completely transparent
2) you can actually poison someone's coordination table and overwrite dst client's ip to redirect packets to yourself. see https://github.com/nosiee/olla/blob/main/olla/src/coordinator/packet.rs#L96
3) the client must periodically send keepalive packets to nodes to maintain the NAT cache

see configs/config.toml for a basic config example and `olla configs/config.toml client/node` to run
the helpers scripts may be usefull to masquerade the traffic
