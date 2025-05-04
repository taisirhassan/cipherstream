use libp2p::Multiaddr;

#[test]
fn test_extract_port_from_multiaddr() {
    // This is a private function in network::mod.rs, but we can test it by recreating it here
    fn extract_port_from_multiaddr(addr: &Multiaddr) -> Option<u16> {
        use libp2p::multiaddr::Protocol;
        
        for proto in addr.iter() {
            if let Protocol::Tcp(port) = proto {
                return Some(port);
            }
        }
        None
    }

    // Test with a simple TCP address
    let addr1: Multiaddr = "/ip4/127.0.0.1/tcp/8080".parse().unwrap();
    assert_eq!(extract_port_from_multiaddr(&addr1), Some(8080));
    
    // Test with a more complex address
    let addr2: Multiaddr = "/ip4/127.0.0.1/tcp/9000/p2p/QmcgpsyWgH8Y8ajJz1Cu72KnS5uo2Aa2LpzU7kinSupNKC".parse().unwrap();
    assert_eq!(extract_port_from_multiaddr(&addr2), Some(9000));
    
    // Test with address that has no TCP component
    let addr3: Multiaddr = "/ip4/127.0.0.1/udp/1234".parse().unwrap();
    assert_eq!(extract_port_from_multiaddr(&addr3), None);
    
    // Test with ephemeral port
    let addr4: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    assert_eq!(extract_port_from_multiaddr(&addr4), Some(0));
}

#[test]
fn test_multiaddr_with_various_protocols() {
    // Test IPv4 address
    let ipv4_addr: Multiaddr = "/ip4/192.168.1.1/tcp/8080".parse().unwrap();
    assert!(ipv4_addr.to_string().contains("/ip4/192.168.1.1"));
    assert!(ipv4_addr.to_string().contains("/tcp/8080"));
    
    // Test IPv6 address
    let ipv6_addr: Multiaddr = "/ip6/::1/tcp/8080".parse().unwrap();
    assert!(ipv6_addr.to_string().contains("/ip6/::1"));
    assert!(ipv6_addr.to_string().contains("/tcp/8080"));
    
    // Test with peer ID
    let peer_addr: Multiaddr = "/ip4/127.0.0.1/tcp/8080/p2p/QmcgpsyWgH8Y8ajJz1Cu72KnS5uo2Aa2LpzU7kinSupNKC".parse().unwrap();
    assert!(peer_addr.to_string().contains("/p2p/"));
}

#[test]
fn test_multiaddr_operations() {
    // Test multiaddr concatenation
    let base_addr: Multiaddr = "/ip4/127.0.0.1".parse().unwrap();
    let tcp_proto: Multiaddr = "/tcp/8080".parse().unwrap();
    
    // Concatenate by creating a new multiaddr from the strings
    let combined = format!("{}{}", base_addr, tcp_proto).parse::<Multiaddr>().unwrap();
    assert_eq!(combined.to_string(), "/ip4/127.0.0.1/tcp/8080");
} 