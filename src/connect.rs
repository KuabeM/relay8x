use serial::prelude::*;
use std::io;
use std::io::{Error, ErrorKind};
use bytes::{BytesMut, BufMut};
use std::rc::Rc;
use std::time::Duration;

// type aliases for relay vecs and card vecs
pub type RelayIndex = Vec<u8>;
pub type CardIndex = Vec<u8>;

pub struct Relay8x {
    // address of the first card, succeding card has +1 and so on
    start_address: u8,
    // struct containing the serial port settings and stuff
    port: Rc<SerialPort>,
}

/// enum for all possbile commands
#[derive(Debug)]
pub enum Relay8xCmdSet {
    // initialisation command
    Init,
    // switch relays on or off
    Set,
    // toggle relays
    Toggle,
    // reset (switch off) relays
    Reset,
}


impl Relay8xCmdSet {
    /// based on command, address and card, returns the command frame for the relay card
    pub fn encode(self, bytes: &mut BytesMut, start_address: u8, card: Option<u8>, relays: Option<&RelayIndex>) -> io::Result<()> {
        match self {
            Relay8xCmdSet::Init =>  {
                let cmd = 1; // init command: 1
                bytes.put_u8(cmd); // first byte: command
                bytes.put_u8(start_address); // second byte: address of card
                bytes.put_u8(0);  // third: dont care
                let checksum = Relay8xCmdSet::checksummed(&bytes[..]); // fourth: XOR
                bytes.put_u8(checksum);
                debug!("Init command: {:?}", &bytes);
            },
            Relay8xCmdSet::Set => {
                let cmd = 6; // command for turning on: 6
                bytes.put_u8(cmd);  // first byte: command
                let address = Relay8xCmdSet::addressed(start_address, card);
                bytes.put_u8(address); // second byte: address of card
                let relay_bin = Relay8xCmdSet::relay_as_u8(relays.unwrap());
                debug!("Relays to set: {:08b}", relay_bin);
                bytes.put_u8(relay_bin); // third byte: data of relays
                let checksum = Relay8xCmdSet::checksummed(&bytes[..]); // fourth: XOR
                bytes.put_u8(checksum);
                debug!("Set command: {:?}", &bytes);
            },
            Relay8xCmdSet::Toggle => {
                let cmd = 8; // command for turning on
                bytes.put_u8(cmd);  // first byte: command
                let address = Relay8xCmdSet::addressed(start_address, card);
                bytes.put_u8(address); // second byte: address of card
                let relay_bin = Relay8xCmdSet::relay_as_u8(relays.unwrap());
                debug!("Relays to set: {:08b}", relay_bin);
                bytes.put_u8(relay_bin); // third byte: data of relays
                let checksum = Relay8xCmdSet::checksummed(&bytes[..]); // fourth: XOR
                bytes.put_u8(checksum);
                debug!("Toggle command: {:?}", &bytes);
            },
            Relay8xCmdSet::Reset => {
                let cmd = 7; // command for turning on
                bytes.put_u8(cmd);
                let address = Relay8xCmdSet::addressed(start_address, card);
                bytes.put_u8(address); // second byte: address of card
                let relay_bin = Relay8xCmdSet::relay_as_u8(relays.unwrap());
                debug!("Relays to set: {:08b}", relay_bin);
                bytes.put_u8(relay_bin); // third byte: data of relays
                let checksum = Relay8xCmdSet::checksummed(&bytes[..]); // fourth: XOR
                bytes.put_u8(checksum);
                debug!("Reset command: {:?}", &bytes);
            },
         }
         Ok(())
    }

    /// calculates the data byte based on the relays to be switchted
    fn relay_as_u8(vec: &RelayIndex) -> u8 {
        let mut relay_bin = 0b00000000;
        vec.iter().rev().for_each(|x| {
            relay_bin |= (1 << (x-1)) as u8; // shift ones to the specified relays
        });
        relay_bin
    }

    /// calculates the XOR checksum for the fourth  byte ot the command
    fn checksummed(x: &[u8]) -> u8 {
        x.iter().fold(0u8, |checksum, elem| {checksum ^ elem})
    }
    
    /// calculates the address for each card based on starting address of first card
    fn addressed(address: u8, card: Option<u8>) -> u8 {
        address+card.unwrap_or(1)-1
    }

}

impl Relay8x {

    /// constructor for a new Relay Card
    pub fn new(device_name: String, address: u8) -> io::Result<Self> {
        
        Ok(Self {
            port: Rc::new(::serial::open(&device_name)?),
            start_address: address,
        })
    }

    /// initialise device with correct params
    /// sets device address, function can be used to re-set it
    pub fn init_device(&mut self) -> io::Result<BytesMut> {

        let port = Rc::get_mut(&mut self.port).unwrap();
        Relay8x::configure_device(port)?;
        
        // init relay card
        let mut cmd = BytesMut::with_capacity(4);
        Relay8xCmdSet::encode(Relay8xCmdSet::Init, &mut cmd, self.start_address, None, None)?;

        port.write(&cmd[..])?;
        debug!("Wrote init message..");
        // in order to read all responses from all connected cards, we have to wait a couple of millis
        //sleep(Duration::from_millis(20));
        // allocate a large ByteMut to get all responses into that buffer at once,
        // now it's enough for five cards
        let mut resp = BytesMut::new();
        loop {
            resp.put_u8(0);
            port.read(&mut resp[..])?;
            debug!("Response init: {:?}", &resp);
            
            if resp.contains(&9) {
                break;
            }
        }

        // checks only the first answer since we know that there is one card for sure
        //Relay8x::check_response(&resp, &cmd)?;
        Ok(cmd)
    }

    /// private function for port settings
    fn configure_device(port: &mut SerialPort) -> io::Result<()> {
        // configure interface with its params, see doc of relay card
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

    /// switch arbitrary relays on
    /// numbers: Vector containing all relay numbers (1..8)
    /// state: true for switching on, false for off
    pub fn set_relays(&mut self, cards: CardIndex, numbers: RelayIndex) -> io::Result<BytesMut> {
        
        let port = Rc::get_mut(&mut self.port).unwrap();
        let start_address = self.start_address;
        // with capacity makes it only working for current relay card, but it ensures the
        // right length
        let mut cmd = BytesMut::with_capacity(4);

        for &card in cards.iter() {
            Relay8xCmdSet::encode(Relay8xCmdSet::Set, &mut cmd, start_address, Some(card), Some(&numbers))?;
            port.write(&cmd[..])?;
            let sent_cmd = cmd.clone();
            port.read(&mut cmd[..])?;
            debug!("Set Relays response: {:?}", cmd);
            Relay8x::check_response(&cmd, &sent_cmd)?;
            cmd.clear();
        }
        Ok(cmd)
    }

    /// switch arbitrary relays off
    /// numbers: Vector containing all relay numbers (1..8)
    /// state: true for switching on, false for off
    pub fn reset_relays(&mut self, cards: CardIndex, numbers: RelayIndex) -> io::Result<BytesMut> {
        
        let port = Rc::get_mut(&mut self.port).unwrap();
        let start_address = self.start_address;
        // with capacity makes it only working for current relay card, but it ensures the
        // right length
        let mut cmd = BytesMut::with_capacity(4);

        for &card in cards.iter() {
            Relay8xCmdSet::encode(Relay8xCmdSet::Reset, &mut cmd, start_address, Some(card), Some(&numbers))?;
            port.write(&cmd[..])?;
            let sent_cmd = cmd.clone();
            port.read(&mut cmd[..])?;
            debug!("Reset Relays response: {:?}", cmd);
            Relay8x::check_response(&cmd, &sent_cmd)?;
            cmd.clear();
        }
        Ok(cmd)
    }

    /// toggle aribtrary relays
    /// numbers: vector containing all relay numbers (1..8)
    pub fn toggle_relays(&mut self, cards: CardIndex, numbers: RelayIndex) -> io::Result<BytesMut> {

        let port = Rc::get_mut(&mut self.port).unwrap();
        let start_address = self.start_address;
        // with capacity makes it only working for current relay card, but it ensures the
        // right length
        let mut cmd = BytesMut::with_capacity(4);
        
        for &card in cards.iter() {
            Relay8xCmdSet::encode(Relay8xCmdSet::Toggle, &mut cmd, start_address, Some(card), Some(&numbers))?;
            port.write(&cmd[..])?;
            let sent_cmd = cmd.clone();
            port.read(&mut cmd[..])?;
            debug!("Toggle Relays response: {:?}", cmd);
            Relay8x::check_response(&cmd, &sent_cmd)?;
            cmd.clear();
        }
        Ok(cmd)
    }

    fn check_response(msg: & BytesMut, sent_msg: &BytesMut) -> io::Result<()> {
        
        // check first byte
        let checker_byte = sent_msg.get(0).unwrap_or(&1);
        let checked_bytes = msg.get(0).unwrap_or(&1);
        if *checked_bytes != !checker_byte  {
            return Err(Error::new(ErrorKind::Other, format!("Bad first byte: is {}, should be {}", checked_bytes, !checker_byte)))
        }
        // second byte: adress
        let resp_addr = msg.get(1).unwrap_or(&0);
        let sent_addr = sent_msg.get(1).unwrap_or(&1);
        // weird error, it prints the same addresses but says there are not equal...
        if resp_addr != sent_addr && *sent_addr != 0u8{
            return Err(Error::new(ErrorKind::Other, format!("Wrong Adress: 0x{:02x} instead of 0x{:02x}", resp_addr, sent_addr)))
        }
        // last byte: XOR
        if *msg.get(3).unwrap_or(&0) != (*msg.get(0).unwrap_or(&1) ^ *msg.get(1).unwrap_or(&0) ^ *msg.get(2).unwrap_or(&0)) {
            return Err(Error::new(ErrorKind::Other, "XOR in last byte is wrong"))
        }
        debug!("Check ok");
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn connect_to_card() {    
        let mut relay = Relay8x::new(String::from("/dev/ttyUSB0"), 1).expect("Failed to connect to device");
        let init_response = relay.init_device().expect("Failed to init device");
        let expected_res = BytesMut::from(vec![254, relay.start_address, 11, 254^relay.start_address^11]);
        assert_eq!(init_response, expected_res);
    }

}
