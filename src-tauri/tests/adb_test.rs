use std::net::{Ipv4Addr, SocketAddrV4};

use adb_client::{server::ADBServer, server_device::ADBServerDevice, ADBDeviceExt};

#[test]
fn test_adb_connect_and_dumpsys() {
    let server_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 5037);
    let ws_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 58526);

    let mut server = ADBServer::new(server_addr);
    match server.connect_device(ws_addr) {
        Ok(()) => eprintln!("connect_device: OK"),
        Err(e) => eprintln!("connect_device (maybe already connected): {e:#}"),
    }
    drop(server);

    let serial = "127.0.0.1:58526".to_string();
    let mut device = ADBServerDevice::new(serial, Some(server_addr));

    let mut buf = Vec::new();
    device
        .shell_command(&"echo hello_test", Some(&mut buf), None)
        .expect("echo should succeed");

    let out = String::from_utf8_lossy(&buf);
    eprintln!("echo: {out:?}");
    assert_eq!(out.trim(), "hello_test");

    let mut buf2 = Vec::new();
    device
        .shell_command(&"dumpsys media_session", Some(&mut buf2), None)
        .expect("dumpsys should succeed");

    let out2 = String::from_utf8_lossy(&buf2);
    eprintln!("dumpsys contains Sessions Stack: {}", out2.contains("Sessions Stack"));
    assert!(out2.contains("Sessions Stack"));
}
