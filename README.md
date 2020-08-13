# Step Sequencer

By Michael Mogenson

## Table of Contents
1. [Target Platform](#target-platform)  
3. [Usage](#usage)  
2. [Description](#description)  
4. [Libraries](#libraries)  
5. [Assumptions](#assumptions)  

## Target Platform

This project was developed on Arch Linux but should run on Windows, macOS, and Linux. ANSI escape codes are used for the command line interface. Windows users may need to use an ANSI aware terminal such as the WSL terminal.

## Usage

Run `cargo build` to build an application named `sequencer`.

Run `cargo test` to run all unit and integration tests.

Run `sequencer -h` or `sequencer --help` to print the following help message:

```
A 4 track 16 step sequencer.
        Runs in real time. Accepts commands over stdin. Prints MIDI data over stdout.

Usage: sequencer <tempo> [--midiout]
        for <tempo> in beats per minute, use --midiout flag to write raw MIDI to stderr.

Commands: Enter one of the following commands plus arguments during execution.
        start                                                          (start sequencer)
        pause                                                          (pause sequencer)
        steps                                            (print current and total steps)
        addnote <track> <step> <pitch> <velocity> <duration>     (add note to sequencer)
                for <track> in 0..3, <step> in 0..15,
                <pitch> in 0..127, <velocity> in 0..127, <duration> in 0..15
        delnote <track> <step> <pitch>                        (remove note in sequencer)
                for <track> in 0..3, <step> in 0..15, <pitch> in 0..127,
        addparam <track> <step> <controller> <value>    (set parameter change for voice)
                for <track> in 0..3, <step> in 0..15,
                <controller> in mod/breath/vol/pan, <value> in 0..127
        delparam <track> <step> <controller>          (clear parameter change for voice)
                for <track> in 0..3, <step> in 0..15, <controller> in mod/breath/vol/pan

```

While running, there is an array of 16 white squares on the left side of the terminal. These represent the 16 steps. The current step is highlighted in black.

To the right of the step array is a command prompt labeled `CMD:` to enter one of the sequencer commands listed above. For example, enter `addnote 1 2 60 127 4` to add a middle C note with a velocity of 127 and duration of 4 steps to step 2 of track 1. Or, enter `addparam 3 4 mod 100` to set a parameter change of type `Modulation` and value 100 for the `Voice` of track 3 on step 4.

To the right of the command prompt is the current event display, labeled `EVT:`. In this section, the sequencer events generated for the current step are printed out using MIDI notation. These events will be `note on`, `note off`, or `controller change` 3-byte MIDI messages. They are rendered as ASCII text for debugging. The sequencer tracks are mapped to MIDI channels 1 to 4.

Use the `--midiout` command flag when starting this program to write raw MIDI messages to `stderr`. These can be redirected to a hardware MIDI interface via `sequencer 60 --midiout 2>/dev/midi00` on a Linux platform.

## Description

This project consists of a `Clock` to generate tick events, a `Sequencer` to store `Track` and `Step` state and generate `Events`, and a command line interface to parse commands from `stdin` and print events to `stdout`.

Initially, a decision about application flow needed to be made. Should the user interface pull events out of the sequencer and the sequencer wait for the next clock tick, or should the clock push ticks to the sequencer and the user interface? The former approach could utilize practices such as using an async executor (to await each new sequencer step), or constructing the sequencer as a generator (that could conform to the iterator API). These designs could be single threaded to prevent concurrent access to data. However, the timing for a polling approach can only be consistent if the sequencer and clock tasks are serviced frequently. If the single thread is working on user input, the sequencer timing may fall behind or jitter. It was assumed that the accuracy and consistency of a step sequencer is the highest priority for a musical instrument. Therefore, this project was designed to move data from the clock source up. The clock, and part of the sequencer, run together in a separate thread. This thread does not yield to ensure that steps happen as close to the beat as possible.

The `SystemClock` implementation for this project polls the current OS system time. It compares the current time to a timestamp one period in the future, when the next tick should occur. After each tick, a shared tick counter is incremented, and the next timestamp is generated. This approach is not efficient. The thread will spend the majority of it's CPU cycles querying the system time. However, the system time resource is available across platforms, which allow this implementation to work on Windows, macOS, and Linux. A `Clock` trait was created so that alternative clock implementations could be used with this sequencer. For example, a hardware timer for an embedded system. The `Clock` trait specifies functions to start, stop, and query the clock state. Additionally, an `on_tick()` method registers a callback to be executed on each clock tick.

The `Sequencer` struct is constructed around a clock type. It uses the `on_tick()` method to evaluate a closure that processes each step and generates events. The sequencer consists of 4 tracks. Each track contains a `Voice` struct that stores current values for `Modulation`, `Breath`, `Volume`, and `Pan` controllers. Additionally, there is an array of 16 `Steps` per track. Each step contains a `note on`, `note off`, and `param` vector. The first vector consists of `Note` items that hold `pitch`, `velocity`, and `duration` values. The second vector contains `pitch` values. Finally, the third vector contains `Param` items that hold a `Controller` type and `value`.

Each track is polyphonic, in the sense that the vector of notes for each step can grow infinitely. However, every note in a step must have a unique pitch. This allows the user to remove a note from a step by specifying the track, step, and pitch values. It also associates a note off event with a single note on event.

The data stored in the sequencer is bounded by the range of MIDI messages, the number of steps, the number of tracks, and the supported controller types. A number of custom data types, such as `u2`, `u4`, `u7`, `Controller`, `Note`, and `Param` were created to ensure that all values inputted into the sequencer and emitted by the sequencer are valid.

The sequencer provides an API for adding and removing notes, setting and clearing parameters, starting and pausing execution, and registering a callback via `on_step()`. This callback is executed on each step with a vector of generated events. The command line interface uses the `on_step()` method to update the user interface with a new sequencer step array and to generate and print MIDI messages.

The command line interface handles parsing command line arguments, creating the sequencer, and reading user commands from `stdin`. Parsing functions were written for the `u2`/`u4`/`u7` bounded integers and `Controller` type that can be chained together to generate a set of command arguments from an inputted string.

## Libraries

This project does not use any crates outside of the standard library. This constraint was chosen to demonstrate my design and implementation ability using only what the language and OS provide. There are a number of crates I would recommended using for parts of this project that provide better performance, a more robust API, and a more audited implementation. These crates include [`nom`](https://github.com/Geal/nom) for string parsing, [`typenum`](https://github.com/paholg/typenum) for bounded integer types, and [`crossbeam`](https://github.com/crossbeam-rs/crossbeam) for cross-thread messaging and data sharing.
)

## Assumptions

- The sequencer should be hard-coded to have 4 tracks and 16 steps. This is not configurable at runtime.
- The remainder of a period is thrown away when `pause()` is called.
- The first step occurs immediately when `start()` is called. The sequencer does not wait for a period.
- A track should be polyphonic. But only one note of each pitch can be played at a single time.
- The application should be cross platform unless there is a hardware reason why a platform cannot be used.
- MIDI data types can be used for note pitch and parameter types.
- Setable parameters are track specific.
- Note duration can be number of steps. A duration cannot be longer than 16 steps.
- A duration of zero emits a note on and note off event at the same time.
- MIDI notation can be used for generated step events.
