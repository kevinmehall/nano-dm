extern crate libusb;
use std::time::Duration;
use std::thread;
use std::process;
use std::io::{self, Write};

fn main() {
    let context = libusb::Context::new().unwrap();
    let device = find_device(&context);
    
    match device {
        Ok(handle) => {
            if let Err(e) = run(handle) {
                println!("{}", e);
                process::exit(2);
            }
        },
        Err(libusb::Error::NotFound) => {
            println!("Device not found");
            process::exit(1);
        }
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    }
}

fn find_device<'a>(context: &'a libusb::Context) -> Result<libusb::DeviceHandle<'a>, libusb::Error> {
    let devices = try!(context.devices());
    devices.iter().find(|device| {
        device.device_descriptor().ok().map_or(false, |desc| desc.vendor_id() == 0x05c6)
    }).map_or(Err(libusb::Error::NotFound), |device| device.open())
}

fn run(mut handle: libusb::DeviceHandle) -> Result<(), libusb::Error> {
    let mut stdout = io::stdout();
    println!("Found device");
    try!(handle.claim_interface(0));
    
    let mut read_buf = [0; 4096];
    
    loop {
        println!("Connecting DMSS");
        
        try!(handle.write_bulk(0x01, &[0x0c, 0x00, 0x00, 0x00,  0x00, 0x47, 0xb8, 0x7e], Duration::from_secs(10)));
        let len = try!(handle.read_bulk(0x81, &mut read_buf, Duration::from_secs(10)));
        let rx = &read_buf[0..len];
        
        if rx == &[0x13, 0x0c, 0x00, 0x00,  0x00, 0x00, 0x72, 0xce, 0x7e] {
            break;
        } else {
            thread::sleep(Duration::from_millis(100));
        }
    }
    
    try!(handle.write_bulk(0x01, &[0x7d, 0x5d, 0x04, 0x34,  0x21, 0x34, 0x21, 0x00,  0x00, 0x1f, 0x00, 0x00,  0x00, 0xab, 0xdf, 0x7e], Duration::from_secs(1)));
    println!("DMSS is connected");
    
    loop {
        let len = try!(handle.read_bulk(0x81, &mut read_buf, Duration::from_secs(0)));

        for packet in Hdlc::new(read_buf[0..len].iter().map(|&x| x)) {
            if packet.len() > 0 {
                parse_packet(&mut stdout, packet).unwrap();
            }
        }
    }
}

fn split_byte(s: &[u8], sep: u8) -> (&[u8], &[u8]) {
    if let Some(pos) = s.iter().position(|&i| i == sep) {
        let (a, b) = s.split_at(pos);
        (a, &b[1..])
    } else {
        (s, &[])
    }
}

fn strip_trailing_newlines(mut s: &[u8]) -> &[u8] {
    while s.ends_with(b"\n") {
        s = &s[0..s.len()-1];
    }
    s
}

fn le32(s: &[u8]) -> u32 {
    (s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24
}

fn le16(s: &[u8]) -> u16 {
    (s[0] as u16) | (s[1] as u16) << 8
}

fn parse_packet(stdout: &mut Write, packet: Vec<u8>) -> io::Result<()> {
    if packet.len() < 24 || packet[0] != 0x79 || packet[2] != 0 {
        print!("[unknown packet: ");
        for b in &packet { print!("{:02x}", b); }
        println!("]");
        return Ok(());
    }
    
    let (header, rest) = packet.split_at(20);
    let timestamp = le32(&header[6..10]);
    let lineno = le16(&header[12..14]);
    let (msg, rest) = split_byte(rest, 0);
    let (file, _) = split_byte(rest, 0);
    
    //for b in &packet { print!("{:02x} ", b); }
    
    try!(write!(stdout, "{:10}: ", timestamp));
    try!(stdout.write_all(strip_trailing_newlines(msg)));
    try!(write!(stdout, " ("));
    try!(stdout.write_all(file));
    try!(write!(stdout, ":{})\n", lineno));
    
    Ok(())
}

/// Iterator over packets in a HDLC-framed sequence of bytes
struct Hdlc<I> {
    input: I,
    end: bool,
}

impl<I> Hdlc<I> where I:Iterator<Item=u8> {
    fn new(input: I) -> Hdlc<I> {
        Hdlc {
            input: input,
            end: false,
        }
    }
    
    fn input_byte(&mut self) -> Option<u8> {
        let x = self.input.next();
        self.end |= x.is_none();
        x
    }
}

impl<I> Iterator for Hdlc<I> where I:Iterator<Item=u8> {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Vec<u8>> {
        if self.end {
            return None;
        }
        
        let mut out = vec![];
        
        loop {
            match self.input_byte() {
                Some(0x7e) | None => break,
                Some(0x7d) => {
                    match self.input_byte() {
                        Some(c) => out.push(c ^ 0x20),
                        None => break,
                    }
                }
                Some(c) => out.push(c),
            };
        }
        
        Some(out)
    }
}