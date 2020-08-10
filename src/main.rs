use sequencer::{
    clock::SystemClock,
    sequencer::Sequencer,
    types::{u2, u4, u7, Controller, Event, Note, Param},
};
use std::convert::TryFrom;
use std::env::args;
use std::io::{self, Write};
use std::num::NonZeroU8;
use std::process::exit;

fn main() {
    if args().len() < 2
        || args().nth(1).unwrap_or_else(|| "-h".to_string()) == "-h"
        || args().nth(1).unwrap_or_else(|| "--help".to_string()) == "--help"
    {
        println!(
            "A {} track {} step sequencer.",
            Sequencer::TRACKS,
            Sequencer::STEPS
        );
        println!("\tRuns in real time. Accepts commands over stdin. Prints MIDI data over stdout.");
        println!();
        println!(
            "Usage: {} <tempo> [--midiout]",
            args().next().unwrap_or_else(|| "sequencer".to_string())
        );
        println!("\tfor <tempo> in beats per minute, use --midiout flag to write raw MIDI to stderr.");
        println!();
        println!("Commands: Enter one of the following commands plus arguments during execution.");
        println!(
            "\tstart                                                          (start sequencer)"
        );
        println!(
            "\tpause                                                          (pause sequencer)"
        );
        println!(
            "\tsteps                                            (print current and total steps)"
        );
        println!(
            "\taddnote <track> <step> <pitch> <velocity> <duration>     (add note to sequencer)"
        );
        println!(
            "\t\tfor <track> in 0..{}, <step> in 0..{},",
            Sequencer::TRACKS - 1,
            Sequencer::STEPS - 1
        );
        println!(
            "\t\t<pitch> in 0..{}, <velocity> in 0..{}, <duration> in 0..{}",
            u7::MAX,
            u7::MAX,
            u4::MAX
        );
        println!(
            "\tdelnote <track> <step> <pitch>                        (remove note in sequencer)"
        );
        println!(
            "\t\tfor <track> in 0..{}, <step> in 0..{}, <pitch> in 0..{},",
            Sequencer::TRACKS - 1,
            Sequencer::STEPS - 1,
            u7::MAX
        );
        println!(
            "\taddparam <track> <step> <controller> <value>    (set parameter change for voice)"
        );
        println!(
            "\t\tfor <track> in 0..{}, <step> in 0..{},",
            Sequencer::TRACKS - 1,
            Sequencer::STEPS - 1
        );
        println!(
            "\t\t<controller> in mod/breath/vol/pan, <value> in 0..{}",
            u7::MAX,
        );
        println!(
            "\tdelparam <track> <step> <controller>          (clear parameter change for voice)"
        );
        println!(
            "\t\tfor <track> in 0..{}, <step> in 0..{}, <controller> in mod/breath/vol/pan",
            Sequencer::TRACKS - 1,
            Sequencer::STEPS - 1,
        );

        exit(0);
    }

    // parse tempo
    let tempo = parse_tempo(args().nth(1)).unwrap_or_else(|error| {
        println!("Error: {}", error);
        exit(-1);
    });

    // parse midiout flag
    let midiout = if let Some(flag) = args().nth(2) {
        flag == "--midiout"
    } else {
        false
    };

    // build sequencer
    let mut sequencer = Sequencer::new()
        .with_tempo(tempo)
        .on_step(move |step, events| {
            print_step(step);
            print_events(events, midiout);
        })
        .build();

    // read commands from stdin
    loop {
        print_prompt();
        parse_command(&mut sequencer).unwrap_or_else(|error| println!("Error: {}", error));
    }
}

fn print_prompt() {
    print!("\x1b[0G");
    print!("{:⬜<1$}", "", Sequencer::STEPS);
    print!(" CMD: ");
    io::stdout().flush().unwrap();
}

fn print_step(step: usize) {
    print!("\x1b[s"); // save cursor location
    print!("\x1b[0G"); // goto beginning of line
    print!("{:⬜<1$}", "", step);
    print!("⬛"); // print 15 white square and 1 black square for current step
    print!("{:⬜<1$}", "", Sequencer::STEPS - step - 1);
    print!("\x1b[u"); // goto saved position
    io::stdout().flush().unwrap();
}

fn print_events(events: Vec<Event>, midiout: bool) {
    print!("\x1b[s"); // save cursor location
    print!("\x1b[0K"); // erase to end of line
    print!(" EVT: "); // print prompt
    let mut midi = [0u8; 3];
    for event in events {
        match event {
            Event::NoteOn {
                channel,
                pitch,
                velocity,
            } => {
                midi[0] = 0x90 | u8::from(channel);
                midi[1] = u8::from(pitch);
                midi[2] = u8::from(velocity);
            }
            Event::NoteOff { channel, pitch } => {
                midi[0] = 0x80 | u8::from(channel);
                midi[1] = u8::from(pitch);
                midi[2] = 0;
            }
            Event::ControllerChange {
                channel,
                controller,
                value,
            } => {
                midi[0] = 0xB0 | u8::from(channel);
                midi[1] = u8::from(controller);
                midi[2] = u8::from(value);
            }
        }
        print!("{:x?}", midi);
        if midiout {
            io::stderr().write_all(&midi).unwrap(); // write raw midi data to stderr
        }
    }
    print!("\x1b[u"); // goto saved position
    io::stdout().flush().unwrap();
}

fn parse_command(sequencer: &mut Sequencer<SystemClock>) -> Result<(), &'static str> {
    let mut command = String::new();

    if io::stdin().read_line(&mut command).is_err() {
        return Err("could not read stdin");
    }

    let mut args = command.trim().split_whitespace();
    match args.next() {
        Some("start") => {
            sequencer.start();
            Ok(())
        }
        Some("pause") => {
            sequencer.pause();
            Ok(())
        }
        Some("steps") => {
            let steps = sequencer.get_steps();
            println!(
                "current step: {} total steps: {}",
                u8::from(steps.0),
                steps.1
            );
            Ok(())
        }
        Some("addnote") => parse_int::<u2>(args.next()).and_then(|track| {
            parse_int::<u4>(args.next()).and_then(|step| {
                parse_int::<u7>(args.next()).and_then(|pitch| {
                    parse_int::<u7>(args.next()).and_then(|velocity| {
                        parse_int::<u4>(args.next()).map(|duration| {
                            sequencer.add_note(
                                track,
                                step,
                                Note {
                                    pitch,
                                    velocity,
                                    duration,
                                },
                            )
                        })
                    })
                })
            })
        }),
        Some("delnote") => parse_int::<u2>(args.next()).and_then(|track| {
            parse_int::<u4>(args.next()).and_then(|step| {
                parse_int::<u7>(args.next())
                    .map(|pitch| sequencer.delete_note(track, step, Note::from_pitch(pitch)))
            })
        }),
        Some("addparam") => parse_int::<u2>(args.next()).and_then(|track| {
            parse_int::<u4>(args.next()).and_then(|step| {
                parse_controller(args.next()).and_then(|controller| {
                    parse_int::<u7>(args.next())
                        .map(|value| sequencer.set_param(track, step, Param { controller, value }))
                })
            })
        }),
        Some("delparam") => parse_int::<u2>(args.next()).and_then(|track| {
            parse_int::<u4>(args.next()).and_then(|step| {
                parse_controller(args.next()).map(|controller| {
                    sequencer.clear_param(track, step, Param::from_controller(controller))
                })
            })
        }),
        _ => Err("invalid command"),
    }
}

fn parse_tempo(arg: Option<String>) -> Result<NonZeroU8, &'static str> {
    if let Some(string) = arg {
        if let Ok(int) = string.parse::<u8>() {
            if let Some(tempo) = NonZeroU8::new(int) {
                Ok(tempo)
            } else {
                Err("tempo cannot be zero")
            }
        } else {
            Err("could not parse tempo")
        }
    } else {
        Err("no tempo provided")
    }
}

fn parse_int<T: TryFrom<u8>>(arg: Option<&str>) -> Result<T, &'static str> {
    if let Some(string) = arg {
        if let Ok(int) = string.parse::<u8>() {
            if let Ok(val) = T::try_from(int) {
                Ok(val)
            } else {
                Err("number is out of bounds")
            }
        } else {
            Err("could not parse arg")
        }
    } else {
        Err("missing argument")
    }
}

fn parse_controller(arg: Option<&str>) -> Result<Controller, &'static str> {
    if let Some(string) = arg {
        match string {
            "mod" => Ok(Controller::Modulation),
            "breath" => Ok(Controller::Breath),
            "vol" => Ok(Controller::Volume),
            "pan" => Ok(Controller::Pan),
            _ => Err("invalid controller"),
        }
    } else {
        Err("missing argument")
    }
}
