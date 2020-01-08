//! Records a WAV file (roughly 4 seconds long) using the default input device and format.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

extern crate anyhow;
extern crate cpal;

//use serenity::voice::{AudioType, AudioSource};
use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use cpal::{StreamId, Device};

use std::io::Read;

use byteorder::{ByteOrder, LittleEndian};

use std::sync::{Arc, Mutex};

use std::collections::VecDeque;

pub struct VCBAudioSource {
    stereo: bool,
    queue: Arc<Mutex<VecDeque<i16>>>,
    device: Device, 
    stream_id: Option<StreamId>,
}

impl Read for VCBAudioSource {
    // vérifier que ça copie bien
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        //println!("VCBAudioSource read appelé.");
        loop {
            let q = self.queue.lock().unwrap();
            if q.len() != 0 {
                break;
            }
        }
        let mut q = self.queue.lock().unwrap();
        let i = q.pop_front().unwrap();
        LittleEndian::write_i16(buf, i);
        Ok(buf.len())
    }
}

impl VCBAudioSource {
    pub fn new(dev_name: String) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .input_devices()
            .expect("No input devices !")
            .find(|x| x.name().unwrap() == dev_name)
            .unwrap();
        let format = device
            .default_input_format()
            .expect("Failed to get default input format");
        Ok(VCBAudioSource {
            queue: Arc::new(Mutex::new(VecDeque::<i16>::new())),
            stereo: (format.channels >= 2),
            stream_id: None, 
            device: device,
        })
    }

    pub fn is_stereo(&self) -> bool {
        self.stereo
    }

    pub fn close(&self) -> Result<(), anyhow::Error> {
        let sid = self.stream_id.clone().unwrap();
        let host = cpal::default_host();
        host.event_loop().destroy_stream(sid);
        Ok(())
    }

    pub fn open(&mut self) -> Result<(), anyhow::Error> {
        let host = cpal::default_host();
        let el = host.event_loop();
        let q = Arc::clone(&self.queue);

        let format = self.device
            .default_input_format()
            .expect("Failed to get default input format");

        let sid = el.build_input_stream(&self.device, &format)?; // attention ici peut-être renvoyer un Result<>
        self.stream_id = Some(sid.clone());
        el.play_stream(sid)?;

        std::thread::spawn(move || {
            el.run(move |id, event| {
                let data = match event {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!("an error occurred on stream {:?}: {}", id, err);
                        return;
                    }
                };

                match data {
                    cpal::StreamData::Input {
                        buffer: cpal::UnknownTypeInputBuffer::U16(buffer),
                    } => {
                        for sample in buffer.iter() {
                            let sample = cpal::Sample::to_i16(sample);
                            q.lock().unwrap().push_back(sample);
                        }
                    }
                    cpal::StreamData::Input {
                        buffer: cpal::UnknownTypeInputBuffer::I16(buffer),
                    } => {
                        for &sample in buffer.iter() {
                            q.lock().unwrap().push_back(sample);
                        }
                    }
                    cpal::StreamData::Input {
                        buffer: cpal::UnknownTypeInputBuffer::F32(buffer),
                    } => {
                        for sample in buffer.iter() {
                            let sample = cpal::Sample::to_i16(sample);
                            q.lock().unwrap().push_back(sample);
                        }
                    }
                    _ => (),
                }
            });
        });
        Ok(())
    }
}
