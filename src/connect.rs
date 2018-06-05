use serial::prelude::*;
use std::io;
use std::time::Duration;
use bytes::{BytesMut, BufMut};
use std::rc::Rc;

pub struct Relay8x {
    device_name: String,
    address: u8,
    port: Rc<SerialPort>,
}

impl Relay8x {

    /// constructor for a new Relay Card
    pub fn new(device_name: String, address: u8) -> io::Result<Self> {
        let device_name = format!("/dev/{}", device_name);
        Ok(Self {
            port: Rc::new(::serial::open(&device_name)?),
            device_name: device_name,
            address: address,
        })
    }

    /// initialise device with correct params
    /// sets device address, function can be used to re-set it
    pub fn init_device(&mut self) -> io::Result<()> {
        self.configure_port()?;
        let port = Rc::get_mut(&mut self.port).unwrap();
        // init relaycard
        let mut cmd = BytesMut::with_capacity(4);
        let cmd_no = 1; // first byte: command init device
        cmd.put_u8(cmd_no);
        cmd.put_u8(self.address); // second byte: address of card
        cmd.put_u8(0);  // third: dont care
        cmd.put_u8(cmd_no ^ self.address ^ 0); // fourth: XOR

        port.write(&cmd[..])?;

        port.read(&mut cmd[..])?;
        debug!("Response: {:?}", cmd);
        // TODO return response and check if ok

        Ok(())
    }

    /// private function for port settings
    fn configure_port(&mut self) -> io::Result<()> {
        let port = Rc::get_mut(&mut self.port).unwrap();

        port.reconfigure(&|settings| {
            settings.set_baud_rate(::serial::Baud19200)?;
            settings.set_char_size(::serial::Bits8);
            settings.set_parity(::serial::ParityNone);
            settings.set_stop_bits(::serial::Stop1);
            settings.set_flow_control(::serial::FlowNone);
            Ok(())
        })?;

        port.set_timeout(Duration::from_millis(1000))?;

        Ok(())
    }

    /// switch one relay on or off
    /// number: 1..8 corresponding to relays X1 to X8 on the board
    /// state: true for switching on, false for off
    pub fn set_relay(&mut self, number: u8, state: bool) -> io::Result<()> {
        let port = Rc::get_mut(&mut self.port).unwrap();

        // cmd message has 4 bytes
        let mut cmd = BytesMut::with_capacity(4);
        let on_off = if state { // on
            6
        } else { // off
            7
        };
        cmd.put_u8(on_off);
        cmd.put_u8(self.address);
        cmd.put_u8(number);
        cmd.put_u8(on_off ^ self.address ^ number);

        port.write(&cmd[..])?;
        // TODO check response

        Ok(())
    }

    /// switch more than one relay on or off
    /// numbers: Vector containing all relay numbers (1..8)
    /// state; true for switching on, false for off
    pub fn set_relays(&mut self, numbers: Vec<u8>, state: bool) -> io::Result<()> {
        let port = Rc::get_mut(&mut self.port).unwrap();
        
        let mut cmd = BytesMut::with_capacity(4);
        let on_off = if state { // on
            6
        } else { // off
            7
        };
        cmd.put_u8(on_off);
        cmd.put_u8(self.address);
        let mut relay_bin = 0b00000000;
        numbers.iter().rev().for_each(|x| {
            relay_bin <<= x;
        });
        cmd.put_u8(relay_bin);
        cmd.put_u8(on_off ^ self.address ^ relay_bin);

        println!("{:?} => {:b}", numbers, relay_bin);

        port.write(&cmd[..])?;
        // TODO check the repsonse

        Ok(())
    }

    pub fn toggle_relays(&mut self, numbers: Vec<u8>) -> io::Result<()> {
        let port = Rc::get_mut(&mut self.port).unwrap();

        let mut cmd = BytesMut::with_capacity(4);
        // toggle is command no 8
        cmd.put_u8(8); 
        cmd.put_u8(self.address);
        let mut relay_bin = 0b00000000;
        numbers.iter().rev().for_each(|x| {
            relay_bin <<= x;
        });
        cmd.put_u8(relay_bin);
        cmd.put_u8(8 ^ self.address ^ relay_bin);

        port.write(&cmd[..])?;
        // check the response

        Ok(())
    }
}