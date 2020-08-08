use crate::clock::{Clock, SystemClock};
use crate::types::{u2, u4, u7, Controller, Event, Note, Param};
use std::cell::RefCell;
use std::convert::TryFrom;
use std::num::NonZeroU8;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default, Debug)]
struct Voice {
    modulation: u7,
    breath: u7,
    volume: u7,
    pan: u7,
}

impl Voice {
    fn set_param(&mut self, param: &Param) {
        match param.controller {
            Controller::Modulation => self.modulation = param.value,
            Controller::Breath => self.breath = param.value,
            Controller::Volume => self.volume = param.value,
            Controller::Pan => self.pan = param.value,
        }
    }
}

#[derive(Debug)]
struct Step {
    note_ons: Vec<Note>, // pitch, velocity, and duration
    note_offs: Vec<u7>,  // just pitch
    params: Vec<Param>,  // controller and value
}

impl Default for Step {
    fn default() -> Self {
        Self {
            note_ons: Vec::new(),
            note_offs: Vec::new(),
            params: Vec::new(),
        }
    }
}

#[derive(Default, Debug)]
struct Track {
    voice: Voice,
    steps: [Step; Sequencer::STEPS],
}

// a 4 track, 16 step sequencer
pub struct Sequencer<Clock> {
    clock: RefCell<Clock>, // implements the Clock trait
    callback: Arc<dyn Fn(usize, Vec<Event>) + Send + Sync>, // on step event
    tracks: Arc<Mutex<[Track; Sequencer::TRACKS]>>, // step data
}

impl Sequencer<SystemClock> {
    pub const STEPS_PER_BEAT: u8 = 4;
    pub const STEPS: usize = u4::MAX as usize + 1;
    pub const TRACKS: usize = u2::MAX as usize + 1;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tempo(&mut self, bpm: NonZeroU8) -> &mut Self {
        // beats per min to steps per min to period in seconds
        let period = 60.0 / bpm.get() as f32 / Self::STEPS_PER_BEAT as f32;
        self.clock
            .get_mut()
            .with_period(Duration::from_secs_f32(period));
        self
    }

    pub fn on_step<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(usize, Vec<Event>) + Send + Sync + 'static,
    {
        self.callback = Arc::new(callback);
        self
    }

    pub fn build(&self) -> Self {
        // Clock is wrapped in an Rc so we can can use Copy to create a new and
        // final Sequencer. This means we have to perform an extra step to
        // request a mutable reference whenever we want to modify a Clock
        // property. However, since the &self Clock reference is dropped after
        // this method, we're guaranteed to only have once Clock reference, so
        // all get_mut() calls will succeed.
        let mut sequencer = Self::default();
        sequencer.callback = self.callback.clone();
        sequencer.clock = self.clock.clone();
        sequencer
    }

    pub fn start(&mut self) {
        if self.clock.borrow().is_running() {
            return; // already running
        }

        // variables to move into closure
        let callback = self.callback.clone();
        let tracks = self.tracks.clone();
        let period = self.clock.borrow().get_period();

        self.clock.get_mut().on_tick(move |tick| {
            let step = if tick == 0 {
                0 // tick should always be > 0, but check anyways
            } else {
                (tick - 1) % Sequencer::STEPS
            };
            let mut events: Vec<Event> = Vec::new();

            // We need mutable access in order to update each Track's Voice
            // params. We could block on the mutex and risk missing Clock ticks
            // if another thread is holding the lock or we could bail and not
            // report events for this step. Instead we will poll the mutex lock
            // for half of a clock period before giving up.
            let now = Instant::now();
            while now.elapsed() < (period / 2) {
                if let Ok(mut tracks) = tracks.try_lock() {
                    for (i, track) in tracks.iter_mut().enumerate() {
                        // channel is same as track number
                        let channel = u4::try_from(i as u8).unwrap();
                        // first do controller_changes, since this will affect
                        // the sound of the Voice for upcoming notes
                        for param in &track.steps[step].params {
                            track.voice.set_param(param);
                            events.push(Event::ControllerChange {
                                channel,
                                controller: param.controller.number(),
                                value: param.value,
                            });
                        }
                        // next do note_offs to clear the vector for this step
                        for pitch in track.steps[step].note_offs.drain(..) {
                            events.push(Event::NoteOff { channel, pitch });
                        }
                        // finally do note_ons and queue up note_offs for later
                        for note in &track.steps[step].note_ons {
                            events.push(Event::NoteOn {
                                channel,
                                pitch: note.pitch,
                                velocity: note.velocity,
                            });
                            // a duration of zero gets an immediate note_off
                            if u8::from(note.duration) == 0 {
                                events.push(Event::NoteOff {
                                    channel,
                                    pitch: note.pitch,
                                });
                            } else {
                                track.steps[(step + usize::from(note.duration)) % Sequencer::STEPS]
                                    .note_offs
                                    .push(note.pitch);
                            }
                        }
                    }
                    break;
                }
            }

            callback(step, events);
        });
        self.clock.get_mut().start();
    }

    pub fn pause(&mut self) {
        self.clock.get_mut().stop();
    }

    pub fn is_running(&self) -> bool {
        self.clock.borrow().is_running()
    }

    // returns current step number from 0 to 15 and total number of steps
    pub fn get_steps(&self) -> (u4, usize) {
        assert_eq!(usize::from(u4::MAX), Sequencer::STEPS - 1);
        let ticks = self.clock.borrow().get_ticks();
        if ticks == 0 {
            return (u4::ZERO, 0);
        }
        (u4::try_from((ticks - 1) % Sequencer::STEPS).unwrap(), ticks)
    }

    // add note to step for track. overwrites an existing note with the same pitch.
    pub fn add_note(&mut self, track: u2, step: u4, note: Note) {
        if let Ok(mut tracks) = self.tracks.lock() {
            let notes = &mut tracks[usize::from(track)].steps[usize::from(step)].note_ons;
            notes.retain(|n| n.pitch != note.pitch);
            notes.push(note);
        }
    }

    // removes a note for step in track by matching pitch. does nothing if not does not exist.
    pub fn delete_note(&mut self, track: u2, step: u4, note: Note) {
        if let Ok(mut tracks) = self.tracks.lock() {
            let notes = &mut tracks[usize::from(track)].steps[usize::from(step)].note_ons;
            notes.retain(|n| n.pitch != note.pitch);
        }
    }

    // adds a parameter change to step for track. overwrites an existing parameter with same controller.
    pub fn set_param(&mut self, track: u2, step: u4, param: Param) {
        if let Ok(mut tracks) = self.tracks.lock() {
            let params = &mut tracks[usize::from(track)].steps[usize::from(step)].params;
            params.retain(|p| p.controller != param.controller);
            params.push(param);
        }
    }

    // removes a parameter change for step in track by matching controller type.
    pub fn clear_param(&mut self, track: u2, step: u4, param: Param) {
        if let Ok(mut tracks) = self.tracks.lock() {
            let params = &mut tracks[usize::from(track)].steps[usize::from(step)].params;
            params.retain(|p| p.controller != param.controller);
        }
    }
}

impl Default for Sequencer<SystemClock> {
    fn default() -> Self {
        Self {
            clock: RefCell::new(SystemClock::default()),
            callback: Arc::new(|_, _| {}),
            tracks: Arc::new(Mutex::new(Default::default())),
        }
    }
}
