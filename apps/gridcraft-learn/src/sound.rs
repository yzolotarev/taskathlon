//! Short feedback sounds, synthesized in code (no audio assets) and played through the
//! desktop's own player. Silent when no player is installed.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Correct,
    Wrong,
    Complete,
    LevelUp,
    Lightning,
    Achievement,
}

const RATE: u32 = 22_050;

fn tone(out: &mut Vec<i16>, freq: f32, ms: u32, volume: f32) {
    let n = RATE * ms / 1000;
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let attack = (i as f32 / 200.0).min(1.0);
        let release = ((n - i) as f32 / (n as f32 * 0.6)).min(1.0);
        let wave = (t * freq * std::f32::consts::TAU).sin() * 0.7 + (t * freq * 2.0 * std::f32::consts::TAU).sin() * 0.3;
        out.push((wave * attack * release * volume * i16::MAX as f32) as i16);
    }
}

fn samples(cue: Cue) -> Vec<i16> {
    let mut s = Vec::new();
    match cue {
        Cue::Correct => {
            tone(&mut s, 784.0, 90, 0.35);
            tone(&mut s, 1046.5, 160, 0.35);
        }
        Cue::Wrong => {
            tone(&mut s, 220.0, 120, 0.3);
            tone(&mut s, 185.0, 180, 0.3);
        }
        Cue::Complete => {
            for f in [523.25, 659.25, 783.99, 1046.5] {
                tone(&mut s, f, 110, 0.35);
            }
            tone(&mut s, 1318.5, 260, 0.35);
        }
        Cue::LevelUp => {
            for f in [659.25, 880.0, 1174.7] {
                tone(&mut s, f, 100, 0.35);
            }
        }
        Cue::Lightning => {
            for f in [1046.5, 1318.5, 1568.0, 2093.0] {
                tone(&mut s, f, 55, 0.3);
            }
        }
        Cue::Achievement => {
            for (f, ms) in [(523.25, 120), (523.25, 80), (783.99, 120), (1046.5, 140), (783.99, 90), (1046.5, 320)] {
                tone(&mut s, f, ms, 0.35);
            }
        }
    }
    s
}

fn wav(samples: &[i16]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    b
}

fn file(cue: Cue) -> Option<PathBuf> {
    let name = match cue {
        Cue::Correct => "correct",
        Cue::Wrong => "wrong",
        Cue::Complete => "complete",
        Cue::LevelUp => "levelup",
        Cue::Lightning => "lightning",
        Cue::Achievement => "achievement",
    };
    let path = std::env::temp_dir().join(format!("gridcraft-learn-{name}-v2.wav"));
    if !path.exists() {
        std::fs::write(&path, wav(&samples(cue))).ok()?;
    }
    Some(path)
}

pub fn play(cue: Cue) {
    let Some(path) = file(cue) else { return };
    std::thread::spawn(move || {
        for player in ["paplay", "pw-play", "aplay"] {
            let mut cmd = std::process::Command::new(player);
            if player == "aplay" {
                cmd.arg("-q");
            }
            let ok = cmd
                .arg(&path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            if ok {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn wav_header_is_consistent() {
        let s = super::samples(super::Cue::Correct);
        let w = super::wav(&s);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 44 + s.len() * 2);
    }
}
