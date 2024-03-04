use osc::{decode, encode};
use std::net::{SocketAddr, UdpSocket};

pub fn create_socket(port: u16) -> UdpSocket {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    UdpSocket::bind(addr).unwrap()
}

pub fn send_osc_msg(addr: &str, msg: Vec<osc::OscType>, socket: &UdpSocket) {
    let bytes = encode(addr, msg);
    socket.send_to(&bytes, "0.0.0.0:30001").unwrap();
}

pub fn rcv_osc_msg(socket: &UdpSocket) -> Vec<(String, Vec<osc::OscType>)> {
    let mut buf = [0; 256];

    let mut result = Vec::new();

    let mut count = 0;

    loop {
        let Ok((amt, _)) = socket.recv_from(&mut buf) else {
            break;
        };

        let Ok((addr, content)) = decode(&buf[..amt]) else {
            break;
        };

        result.push((addr, content));

        count += 1;
        if count == 3 {
            loop {
                let Ok(_) = socket.recv_from(&mut buf) else {
                    break;
                };
            }
            break;
        }
    }

    result
}
