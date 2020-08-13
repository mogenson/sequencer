use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

#[test]
fn notes() {
    Command::new("cargo").arg("build").output().unwrap(); // build first
    let mut sequencer = Command::new("target/debug/sequencer")
        .args(&["1", "--midiout"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();

    let stdin = sequencer.stdin.as_mut().unwrap();
    let stderr = sequencer.stderr.as_mut().unwrap();

    stdin.write_all("addnote 0 0 1 1 0\n".as_bytes()).unwrap();
    stdin.write_all("start\n".as_bytes()).unwrap();

    sleep(Duration::from_millis(10));
    let mut buf = [0u8; 6];
    stderr.read_exact(&mut buf).unwrap();
    sequencer.kill().unwrap();

    // one note_on and matching note_off
    assert_eq!([0x90, 0x01, 0x01, 0x80, 0x01, 0x00], buf);
}

#[test]
fn params() {
    Command::new("cargo").arg("build").output().unwrap(); // build first
    let mut sequencer = Command::new("target/debug/sequencer")
        .args(&["1", "--midiout"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();

    let stdin = sequencer.stdin.as_mut().unwrap();
    let stderr = sequencer.stderr.as_mut().unwrap();

    stdin
        .write_all("addparam 0 0 mod 127\n".as_bytes())
        .unwrap();
    stdin.write_all("start\n".as_bytes()).unwrap();

    sleep(Duration::from_millis(10));
    let mut buf = [0u8; 3];
    stderr.read_exact(&mut buf).unwrap();
    sequencer.kill().unwrap();

    // one controller change
    assert_eq!([0xB0, 0x01, 0x7F], buf);
}
