use rosc::{encoder, OscError, OscMessage, OscPacket, OscType};
use std::{
    error::Error,
    net::{SocketAddr, UdpSocket},
};

pub fn decode(buf: &[u8]) -> Result<(String, Vec<OscType>), OscError> {
    let (_, packet) = rosc::decoder::decode_udp(buf).unwrap();
    match packet {
        OscPacket::Message(msg) => Ok((msg.addr, msg.args)),
        _ => Err(OscError::Unimplemented),
    }
}

pub fn encode(addr: &str, args: Vec<OscType>) -> Vec<u8> {
    encoder::encode(&OscPacket::Message(OscMessage {
        addr: addr.to_string(),
        args,
    }))
    .unwrap()
}
pub fn create_socket(port: u16) -> Result<UdpSocket, Box<dyn Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let res = UdpSocket::bind(addr)?;
    Ok(res)
}

pub fn send_osc_msg(addr: &str, msg: Vec<rosc::OscType>, socket: &UdpSocket) {
    let bytes = encode(addr, msg);
    socket.send_to(&bytes, "0.0.0.0:30001").unwrap();
}

pub fn rcv_osc_msg(socket: &UdpSocket) -> Vec<(String, Vec<rosc::OscType>)> {
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
