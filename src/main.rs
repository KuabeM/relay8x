extern crate serial;
extern crate docopt;
extern crate env_logger;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bytes;
#[macro_use]
extern crate log;
#[macro_use]
extern crate common_failures;
#[macro_use]
extern crate failure;

use docopt::Docopt;
use common_failures::prelude::*;

mod connect;
use connect::{Relay8x, RelayIndex};

const USAGE: &'static str = "
relais8x

Usage:
  relais8x set --dev=<dev> [--relay=<relay> ...] <state>
  relais8x toggle --dev=<dev> [--relay=<relay> ...]
  relais8x reset --dev=<dev> [--relay=<relay> ...]
  relais8x (-h | --help)
  relais8x (-v | --version)
  
Commands:
  set   set specified relay 'on' or 'off', if no relay number is given all relays are set
  toggle    toggle specified relay,  if no relay number is given all relays are toggeled
  reset switch all or just one relay off to reach defined state again

Options:
  -h --help         Show this screen.
  -v --version      Show version.
  --dev=<dev>       path to serial device, e.g. /dev/ttyUSB0
  --relay=<relay>   address of relays (1..8) parsed as row of numbers [default: 1 2 3 4 5 6 7 8]
";

#[derive(Debug, Deserialize)]
struct Args {
    cmd_set: bool,
    cmd_toggle: bool,
    cmd_reset: bool,
    flag_dev: String,
    flag_version: bool,
    flag_help: bool,
    flag_relay: Option<RelayIndex>,
    arg_state: String,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");

fn run() -> Result<()> {

    env_logger::init();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());  
    // set default relay vec

    // check arguments
    if args.flag_version {
        println!("{}: {}", NAME, VERSION);
        Ok(())
    } else if args.flag_help {
        println!("{}", USAGE);
        Ok(())
    } else if args.cmd_set {
        // open device, address of relay is always 1 as for now
        let mut relay = Relay8x::new(args.flag_dev, 1)?;
        relay.init_device()?;
        // map state argument to bool, use false as default
        let state = match args.arg_state.as_ref() {
            "on" => true,
            "off" => false,
            _ => bail!("Failed to determine state '{}'. Use either 'on' or 'off'", args.arg_state),
        };
        // if flag is none, all relays should be set
        let relay_numbers = args.flag_relay.unwrap_or_default();
        // do the switching
        relay.set_relays(relay_numbers, state)?;
        Ok(())

    } else if args.cmd_toggle {
        // open device
        let mut relay = Relay8x::new(args.flag_dev, 1)?;
        relay.init_device()?;
        // if flag is none, all relays should be toggeled
        let relay_numbers = args.flag_relay.unwrap_or_default();
        // do the toggle
        relay.toggle_relays(relay_numbers)?;
        Ok(())
    } else if args.cmd_reset {
        // open device
        let mut relay = Relay8x::new(args.flag_dev, 1)?;
        relay.init_device()?;
        // if flag is none, all relays should be reset
        let relay_numbers = args.flag_relay.unwrap_or_default();
        // do the switching, false = off
        relay.set_relays(relay_numbers, false)?;
        Ok(())
    } else {
        println!("I don't know what you want to do..");
        Ok(())
    }
}


quick_main!(run);