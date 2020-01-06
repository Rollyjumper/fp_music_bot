//! Records a WAV file (roughly 4 seconds long) using the default input device and format.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

extern crate anyhow;
extern crate cpal;

//use serenity::voice::{AudioType, AudioSource};
use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use cpal::{EventLoop, StreamId};

use std::io::Read;

use byteorder::{ByteOrder, LittleEndian};

use std::sync::{Arc, Mutex};

use std::collections::VecDeque;

pub struct VCBAudioSource {
    stereo: bool,
    queue: Arc<Mutex<VecDeque<i16>>>,
    event_loop: Arc<Mutex<EventLoop>>,
    stream_id: StreamId,
}

impl Read for VCBAudioSource {
    // vérifier que ça copie bien
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        //println!("VCBAudioSource read appelé.");
        let mut q = self.queue.lock().unwrap();
        match q.len() {
            0 => {
                // normalement ça ne devrait pas être le cas
                buf.copy_from_slice(&[0, 0]);
                //println!("vide!");
                Ok(2)
            }
            _ => {
                //println!("juste avant copy_from_slice.. buf.len = {}, queue.len = {}", buf.len(), q.len());
                //buf.copy_from_slice(&q[..buf.len()]);
                //println!("juste après copy_from_slice : buf = {:?}", buf);
                //q.remove(0);
                //q.remove(0);
                // println!("size of buf = {}", buf.len());
                // size of buf = 2 always ? 
                let i = q.pop_front().unwrap();
                LittleEndian::write_i16(buf, i);
                Ok(buf.len())
            }
        }
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

        let event_loop = Arc::new(Mutex::new(host.event_loop()));
        let stream_id = event_loop
            .lock()
            .unwrap()
            .build_input_stream(&device, &format)?; // attention ici peut-être renvoyer un Result<>
        Ok(VCBAudioSource {
            queue: Arc::new(Mutex::new(VecDeque::<i16>::new())),
            stereo: (format.channels >= 2),
            event_loop: event_loop,
            stream_id: stream_id,
        })
    }

    pub fn is_stereo(&self) -> bool {
        self.stereo
    }

    pub fn close(&self) -> Result<(), anyhow::Error> {
        let sid = self.stream_id.clone();
        self.event_loop.lock().unwrap().destroy_stream(sid);
        Ok(())
    }

    pub fn open(&self) -> Result<(), anyhow::Error> {
        let sid = self.stream_id.clone();
        let el = Arc::clone(&self.event_loop);
        let q = Arc::clone(&self.queue);

        self.event_loop.lock().unwrap().play_stream(sid)?;

        std::thread::spawn(move || {
            el.lock().unwrap().run(move |id, event| {
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
                            let mut q2 = q.lock().unwrap();
                            q2.push_back(sample);
                        }
                    }
                    cpal::StreamData::Input {
                        buffer: cpal::UnknownTypeInputBuffer::I16(buffer),
                    } => {
                        for &sample in buffer.iter() {
                            let mut q2 = q.lock().unwrap();
                            q2.push_back(sample);
                        }
                    }
                    cpal::StreamData::Input {
                        buffer: cpal::UnknownTypeInputBuffer::F32(buffer),
                    } => {
                        for sample in buffer.iter() {
                            let sample = cpal::Sample::to_i16(sample);
                            let mut q2 = q.lock().unwrap();
                            q2.push_back(sample);
                        }
                    }
                    _ => (),
                }
            });
        });
        Ok(())
    }
}
